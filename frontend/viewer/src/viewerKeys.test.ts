// @vitest-environment node
import {indexedDB as fakeIndexedDB} from "fake-indexeddb";
import {afterEach, beforeAll, describe, expect, it, vi} from "vitest";
import {
	clearViewerPrivateKey,
	createViewerKeys,
	loadViewerPrivateKey,
	storeViewerPrivateKey,
	unwrapViewerPrivateKey,
} from "./viewerKeys";

const EXPORT_KEY = "c2VjcmV0LWV4cG9ydC1rZXktZnJvbS1vcGFxdWUtNjQtYnl0ZXMtbG9uZy0xMjM0NTY3ODkw";
const OTHER_EXPORT_KEY = "b3RoZXItZXhwb3J0LWtleS1mcm9tLW9wYXF1ZS02NC1ieXRlcy1sb25nLTEyMzQ1Njc4OTA";

function base64UrlLength(text: string): number {
	return Math.floor((text.length * 3) / 4);
}

async function sharedSecret(privateKey: CryptoKey, publicKey: CryptoKey): Promise<string> {
	const bits = await crypto.subtle.deriveBits({name: "X25519", public: publicKey}, privateKey, 256);
	return Buffer.from(bits).toString("hex");
}

describe("createViewerKeys", () => {
	it("returns a raw 32-byte X25519 public key in base64url", async () => {
		const keys = await createViewerKeys(EXPORT_KEY);

		expect(keys.publicKey).toMatch(/^[A-Za-z0-9_-]+$/);
		expect(base64UrlLength(keys.publicKey)).toBe(32);
	});

	it("keeps only a non-extractable private key", async () => {
		const keys = await createViewerKeys(EXPORT_KEY);

		expect(keys.privateKey.extractable).toBe(false);
		expect(keys.privateKey.type).toBe("private");
		await expect(crypto.subtle.exportKey("pkcs8", keys.privateKey)).rejects.toThrow();
	});

	it("seals a private key that matches the public key and opens with the same export key", async () => {
		const keys = await createViewerKeys(EXPORT_KEY);
		const unwrapped = await unwrapViewerPrivateKey(keys.encryptedPrivateKey, EXPORT_KEY);

		const peer = await crypto.subtle.generateKey({name: "X25519"}, false, ["deriveBits"]);
		if (!("privateKey" in peer)) throw new Error("expected a key pair");
		const viewerPublic = await crypto.subtle.importKey(
			"raw",
			Buffer.from(keys.publicKey, "base64url"),
			{name: "X25519"},
			true,
			[],
		);

		expect(await sharedSecret(unwrapped, peer.publicKey)).toBe(
			await sharedSecret(peer.privateKey, viewerPublic),
		);
	});

	it("produces a different sealed key every time", async () => {
		const first = await createViewerKeys(EXPORT_KEY);
		const second = await createViewerKeys(EXPORT_KEY);

		expect(first.encryptedPrivateKey).not.toBe(second.encryptedPrivateKey);
	});
});

describe("unwrapViewerPrivateKey", () => {
	it("rejects a different export key", async () => {
		const keys = await createViewerKeys(EXPORT_KEY);

		await expect(
			unwrapViewerPrivateKey(keys.encryptedPrivateKey, OTHER_EXPORT_KEY),
		).rejects.toThrow();
	});

	it("rejects a tampered sealed key", async () => {
		const keys = await createViewerKeys(EXPORT_KEY);
		const sealed = Buffer.from(keys.encryptedPrivateKey, "base64url");
		const last = sealed.length - 1;
		sealed.writeUInt8(sealed.readUInt8(last) ^ 1, last);

		await expect(
			unwrapViewerPrivateKey(sealed.toString("base64url"), EXPORT_KEY),
		).rejects.toThrow();
	});
});

describe("private key storage", () => {
	beforeAll(() => {
		vi.stubGlobal("indexedDB", fakeIndexedDB);
	});

	afterEach(async () => {
		await clearViewerPrivateKey();
	});

	it("is empty before anything is stored", async () => {
		expect(await loadViewerPrivateKey()).toBeNull();
	});

	it("returns the stored key, still non-extractable", async () => {
		const keys = await createViewerKeys(EXPORT_KEY);

		await storeViewerPrivateKey(keys.privateKey);
		const loaded = await loadViewerPrivateKey();

		expect(loaded).toBeInstanceOf(CryptoKey);
		expect(loaded?.extractable).toBe(false);
		expect(loaded?.algorithm.name).toBe("X25519");
	});

	it("forgets the key once cleared", async () => {
		const keys = await createViewerKeys(EXPORT_KEY);
		await storeViewerPrivateKey(keys.privateKey);

		await clearViewerPrivateKey();

		expect(await loadViewerPrivateKey()).toBeNull();
	});
});
