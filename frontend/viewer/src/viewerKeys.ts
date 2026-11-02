// The viewer's X25519 key pair. The server stores the public key in plain and
// the private key only sealed under a key derived from the OPAQUE export key,
// which is computed from the password in this browser and never sent anywhere.
//
// Changing the password later only re-seals this one private key under the new
// export key; nothing encrypted to the public key has to be touched.

const KEY_ALGORITHM = {name: "X25519"};
const WRAP_INFO = new TextEncoder().encode("creators-reward/v1/viewer-private-key-wrap");
const IV_LENGTH = 12;

const DB_NAME = "creators-reward";
const DB_STORE = "keys";
const PRIVATE_KEY_ENTRY = "viewer-private-key";

export interface RegisteredViewerKeys {
	publicKey: string;
	encryptedPrivateKey: string;
	privateKey: CryptoKey;
}

function toBase64Url(bytes: Uint8Array): string {
	let binary = "";
	for (const byte of bytes) binary += String.fromCharCode(byte);
	return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

export function fromBase64Url(text: string): Uint8Array<ArrayBuffer> {
	const base64 = text.replaceAll("-", "+").replaceAll("_", "/");
	const binary = atob(base64.padEnd(Math.ceil(base64.length / 4) * 4, "="));
	return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

async function wrappingKey(exportKey: string): Promise<CryptoKey> {
	const material = await crypto.subtle.importKey(
		"raw",
		fromBase64Url(exportKey),
		"HKDF",
		false,
		["deriveKey"],
	);
	return crypto.subtle.deriveKey(
		{name: "HKDF", hash: "SHA-256", salt: new Uint8Array(0), info: WRAP_INFO},
		material,
		{name: "AES-GCM", length: 256},
		false,
		["wrapKey", "unwrapKey"],
	);
}

export async function unwrapViewerPrivateKey(
	encryptedPrivateKey: string,
	exportKey: string,
): Promise<CryptoKey> {
	const sealed = fromBase64Url(encryptedPrivateKey);
	return crypto.subtle.unwrapKey(
		"pkcs8",
		sealed.slice(IV_LENGTH),
		await wrappingKey(exportKey),
		{name: "AES-GCM", iv: sealed.slice(0, IV_LENGTH)},
		KEY_ALGORITHM,
		false,
		["deriveBits"],
	);
}

export async function createViewerKeys(exportKey: string): Promise<RegisteredViewerKeys> {
	const pair = await crypto.subtle.generateKey(KEY_ALGORITHM, true, ["deriveBits"]);
	if (!("privateKey" in pair)) {
		throw new Error("X25519 key generation did not return a key pair");
	}

	const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));
	const wrapped = new Uint8Array(
		await crypto.subtle.wrapKey("pkcs8", pair.privateKey, await wrappingKey(exportKey), {
			name: "AES-GCM",
			iv,
		}),
	);
	const sealed = new Uint8Array(IV_LENGTH + wrapped.length);
	sealed.set(iv);
	sealed.set(wrapped, IV_LENGTH);
	const encryptedPrivateKey = toBase64Url(sealed);

	return {
		publicKey: toBase64Url(new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey))),
		encryptedPrivateKey,
		// Round-trip through the same path login uses: proves the sealed key
		// opens, and leaves only a non-extractable copy in memory.
		privateKey: await unwrapViewerPrivateKey(encryptedPrivateKey, exportKey),
	};
}

function idbRequest<T>(request: IDBRequest<T>): Promise<T> {
	return new Promise((resolve, reject) => {
		request.addEventListener("success", () => resolve(request.result));
		request.addEventListener("error", () => reject(request.error));
	});
}

async function withStore<T>(
	mode: IDBTransactionMode,
	action: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
	const open = indexedDB.open(DB_NAME, 1);
	open.addEventListener("upgradeneeded", () => open.result.createObjectStore(DB_STORE));
	const db = await idbRequest(open);
	try {
		const transaction = db.transaction(DB_STORE, mode);
		const result = idbRequest(action(transaction.objectStore(DB_STORE)));
		await new Promise<void>((resolve, reject) => {
			transaction.addEventListener("complete", () => resolve());
			transaction.addEventListener("abort", () => reject(transaction.error));
		});
		return await result;
	} finally {
		db.close();
	}
}

// IndexedDB stores the CryptoKey itself (structured clone), so a
// non-extractable key stays non-extractable: page script can use it but can
// never read its bytes.
export async function storeViewerPrivateKey(privateKey: CryptoKey): Promise<void> {
	await withStore("readwrite", (store) => store.put(privateKey, PRIVATE_KEY_ENTRY));
}

export async function loadViewerPrivateKey(): Promise<CryptoKey | null> {
	const value: unknown = await withStore("readonly", (store) => store.get(PRIVATE_KEY_ENTRY));
	return value instanceof CryptoKey ? value : null;
}

export async function clearViewerPrivateKey(): Promise<void> {
	await withStore("readwrite", (store) => store.delete(PRIVATE_KEY_ENTRY));
}
