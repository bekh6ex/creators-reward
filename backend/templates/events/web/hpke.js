// HPKE (RFC 9180) sender, base mode, single shot, for the one suite the
// viewer SPA opens: DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, AES-256-GCM.
// Loaded by the events iframe, which seals enabled donations in the browser
// so the site never reaches the server.
//
// The receiver half is frontend/viewer/src/hpke.ts. Its test seals with this
// file and opens with that one; that file is in turn tested against a record
// sealed by rust-hpke, so this sender is pinned to a reference implementation.

const encoder = new TextEncoder();
const EMPTY = new Uint8Array(0);

const KEM_ID = 0x0020; // DHKEM(X25519, HKDF-SHA256)
const KDF_ID = 0x0001; // HKDF-SHA256
const AEAD_ID = 0x0002; // AES-256-GCM
const HASH_LENGTH = 32;
const KEY_LENGTH = 32;
const NONCE_LENGTH = 12;
const MODE_BASE = 0x00;

const VERSION_LABEL = encoder.encode("HPKE-v1");

/**
 * @param {number} value
 * @param {number} length
 */
function i2osp(value, length) {
	const bytes = new Uint8Array(length);
	for (let i = length - 1, rest = value; i >= 0; i--, rest >>= 8) bytes[i] = rest & 0xff;
	return bytes;
}

/** @param {Uint8Array[]} parts */
function concat(...parts) {
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

/**
 * @param {Uint8Array<ArrayBuffer>} key
 * @param {Uint8Array<ArrayBuffer>} data
 */
async function hmacSha256(key, data) {
	const hmacKey = await crypto.subtle.importKey(
		"raw",
		key,
		{name: "HMAC", hash: "SHA-256"},
		false,
		["sign"],
	);
	return new Uint8Array(await crypto.subtle.sign("HMAC", hmacKey, data));
}

// An empty salt means HashLen zero bytes (RFC 5869); WebCrypto rejects an
// empty HMAC key, so spell it out.
/**
 * @param {Uint8Array<ArrayBuffer>} salt
 * @param {Uint8Array<ArrayBuffer>} ikm
 */
function extract(salt, ikm) {
	return hmacSha256(salt.length > 0 ? salt : new Uint8Array(HASH_LENGTH), ikm);
}

// Every output here is at most HashLen, so a single HMAC block suffices.
/**
 * @param {Uint8Array<ArrayBuffer>} prk
 * @param {Uint8Array} info
 * @param {number} length
 */
async function expand(prk, info, length) {
	return (await hmacSha256(prk, concat(info, Uint8Array.of(1)))).slice(0, length);
}

/**
 * @param {Uint8Array} suiteId
 * @param {Uint8Array<ArrayBuffer>} salt
 * @param {string} label
 * @param {Uint8Array} ikm
 */
function labeledExtract(suiteId, salt, label, ikm) {
	return extract(salt, concat(VERSION_LABEL, suiteId, encoder.encode(label), ikm));
}

/**
 * @param {Uint8Array} suiteId
 * @param {Uint8Array<ArrayBuffer>} prk
 * @param {string} label
 * @param {Uint8Array} info
 * @param {number} length
 */
function labeledExpand(suiteId, prk, label, info, length) {
	return expand(
		prk,
		concat(i2osp(length, 2), VERSION_LABEL, suiteId, encoder.encode(label), info),
		length,
	);
}

/** @param {Uint8Array<ArrayBuffer>} recipientPublicKey */
async function encap(recipientPublicKey) {
	const recipient = await crypto.subtle.importKey(
		"raw",
		recipientPublicKey,
		{name: "X25519"},
		false,
		[],
	);
	const ephemeral = /** @type {CryptoKeyPair} */ (
		await crypto.subtle.generateKey({name: "X25519"}, true, ["deriveBits"])
	);
	const enc = new Uint8Array(await crypto.subtle.exportKey("raw", ephemeral.publicKey));
	const dh = new Uint8Array(
		await crypto.subtle.deriveBits({name: "X25519", public: recipient}, ephemeral.privateKey, 256),
	);
	const eaePrk = await labeledExtract(KEM_SUITE_ID, EMPTY, "eae_prk", dh);
	const sharedSecret = await labeledExpand(
		KEM_SUITE_ID,
		eaePrk,
		"shared_secret",
		concat(enc, recipientPublicKey),
		HASH_LENGTH,
	);
	return {enc, sharedSecret};
}

/**
 * @param {Uint8Array<ArrayBuffer>} sharedSecret
 * @param {Uint8Array} info
 */
async function keySchedule(sharedSecret, info) {
	const pskIdHash = await labeledExtract(HPKE_SUITE_ID, EMPTY, "psk_id_hash", EMPTY);
	const infoHash = await labeledExtract(HPKE_SUITE_ID, EMPTY, "info_hash", info);
	const context = concat(Uint8Array.of(MODE_BASE), pskIdHash, infoHash);
	const secret = await labeledExtract(HPKE_SUITE_ID, sharedSecret, "secret", EMPTY);
	return {
		key: await labeledExpand(HPKE_SUITE_ID, secret, "key", context, KEY_LENGTH),
		baseNonce: await labeledExpand(HPKE_SUITE_ID, secret, "base_nonce", context, NONCE_LENGTH),
	};
}

/**
 * Seals `plaintext` to the raw 32-byte X25519 `recipientPublicKey`, with empty
 * AAD. Returns `enc(32) ‖ ciphertext`.
 *
 * @param {Uint8Array<ArrayBuffer>} recipientPublicKey
 * @param {Uint8Array<ArrayBuffer>} plaintext
 * @param {Uint8Array} info
 * @returns {Promise<Uint8Array<ArrayBuffer>>}
 */
export async function seal(recipientPublicKey, plaintext, info) {
	const {enc, sharedSecret} = await encap(recipientPublicKey);
	const {key, baseNonce} = await keySchedule(sharedSecret, info);
	const aesKey = await crypto.subtle.importKey("raw", key, "AES-GCM", false, ["encrypt"]);
	// Single shot: the only message uses sequence number 0, whose nonce is the
	// base nonce itself.
	const ciphertext = new Uint8Array(
		await crypto.subtle.encrypt({name: "AES-GCM", iv: baseNonce}, aesKey, plaintext),
	);
	return concat(enc, ciphertext);
}
