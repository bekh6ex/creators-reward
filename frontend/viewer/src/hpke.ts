// HPKE (RFC 9180) receiver, base mode, for the one suite the server seals
// with: DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, AES-256-GCM. Built on
// WebCrypto so the viewer's private key stays a non-extractable CryptoKey:
// it is only ever used for X25519 deriveBits, never read.
//
// Interop with the server's rust-hpke is pinned by hpke.test.ts, which opens
// a record the server sealed.

type Bytes = Uint8Array<ArrayBuffer>;

const encoder = new TextEncoder();
const EMPTY: Bytes = new Uint8Array(0);

const KEM_ID = 0x0020; // DHKEM(X25519, HKDF-SHA256)
const KDF_ID = 0x0001; // HKDF-SHA256
const AEAD_ID = 0x0002; // AES-256-GCM
const HASH_LENGTH = 32;
const KEY_LENGTH = 32;
const NONCE_LENGTH = 12;
const ENC_LENGTH = 32;
const MODE_BASE = 0x00;

const VERSION_LABEL = encoder.encode("HPKE-v1");

function i2osp(value: number, length: number): Bytes {
	const bytes = new Uint8Array(length);
	for (let i = length - 1, rest = value; i >= 0; i--, rest >>= 8) bytes[i] = rest & 0xff;
	return bytes;
}

function concat(...parts: Uint8Array[]): Bytes {
	const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
	let offset = 0;
	for (const part of parts) {
		out.set(part, offset);
		offset += part.length;
	}
	return out;
}

const KEM_SUITE_ID = concat(encoder.encode("KEM"), i2osp(KEM_ID, 2));
const HPKE_SUITE_ID = concat(
	encoder.encode("HPKE"),
	i2osp(KEM_ID, 2),
	i2osp(KDF_ID, 2),
	i2osp(AEAD_ID, 2),
);

async function hmacSha256(key: Bytes, data: Bytes): Promise<Bytes> {
	const hmacKey = await crypto.subtle.importKey(
		"raw",
		key,
		{name: "HMAC", hash: "SHA-256"},
		false,
		["sign"],
	);
	return new Uint8Array(await crypto.subtle.sign("HMAC", hmacKey, data));
}

// HKDF-Extract and HKDF-Expand separately: WebCrypto only offers them fused,
// and HPKE needs each on its own. An empty salt means HashLen zero bytes
// (RFC 5869); WebCrypto rejects an empty HMAC key, so spell it out.
function extract(salt: Bytes, ikm: Bytes): Promise<Bytes> {
	return hmacSha256(salt.length > 0 ? salt : new Uint8Array(HASH_LENGTH), ikm);
}

// Every output here is at most HashLen, so a single HMAC block suffices.
async function expand(prk: Bytes, info: Bytes, length: number): Promise<Bytes> {
	return (await hmacSha256(prk, concat(info, Uint8Array.of(1)))).slice(0, length);
}

function labeledExtract(suiteId: Bytes, salt: Bytes, label: string, ikm: Bytes) {
	return extract(salt, concat(VERSION_LABEL, suiteId, encoder.encode(label), ikm));
}

function labeledExpand(suiteId: Bytes, prk: Bytes, label: string, info: Bytes, length: number) {
	return expand(
		prk,
		concat(i2osp(length, 2), VERSION_LABEL, suiteId, encoder.encode(label), info),
		length,
	);
}

async function decap(enc: Bytes, privateKey: CryptoKey, recipientPublicKey: Bytes) {
	const ephemeral = await crypto.subtle.importKey("raw", enc, {name: "X25519"}, false, []);
	const dh = new Uint8Array(
		await crypto.subtle.deriveBits({name: "X25519", public: ephemeral}, privateKey, 256),
	);
	const eaePrk = await labeledExtract(KEM_SUITE_ID, EMPTY, "eae_prk", dh);
	return labeledExpand(
		KEM_SUITE_ID,
		eaePrk,
		"shared_secret",
		concat(enc, recipientPublicKey),
		HASH_LENGTH,
	);
}

async function keySchedule(sharedSecret: Bytes, info: Bytes) {
	const pskIdHash = await labeledExtract(HPKE_SUITE_ID, EMPTY, "psk_id_hash", EMPTY);
	const infoHash = await labeledExtract(HPKE_SUITE_ID, EMPTY, "info_hash", info);
	const context = concat(Uint8Array.of(MODE_BASE), pskIdHash, infoHash);
	const secret = await labeledExtract(HPKE_SUITE_ID, sharedSecret, "secret", EMPTY);
	return {
		key: await labeledExpand(HPKE_SUITE_ID, secret, "key", context, KEY_LENGTH),
		baseNonce: await labeledExpand(HPKE_SUITE_ID, secret, "base_nonce", context, NONCE_LENGTH),
	};
}

// Opens a single-shot `enc ‖ ciphertext` record with empty AAD. Throws if the
// record was not sealed to this key pair under this `info`, or was altered.
export async function openSealed(
	privateKey: CryptoKey,
	recipientPublicKey: Bytes,
	sealed: Bytes,
	info: Bytes,
): Promise<Bytes> {
	if (sealed.length < ENC_LENGTH) throw new Error("sealed record is too short");
	const sharedSecret = await decap(sealed.slice(0, ENC_LENGTH), privateKey, recipientPublicKey);
	const {key, baseNonce} = await keySchedule(sharedSecret, info);
	const aesKey = await crypto.subtle.importKey("raw", key, "AES-GCM", false, ["decrypt"]);
	// Single-shot: the first and only message uses sequence number 0, whose
	// nonce is the base nonce itself.
	return new Uint8Array(
		await crypto.subtle.decrypt({name: "AES-GCM", iv: baseNonce}, aesKey, sealed.slice(ENC_LENGTH)),
	);
}
