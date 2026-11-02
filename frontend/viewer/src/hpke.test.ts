// @vitest-environment node
import {describe, expect, it} from "vitest";
import {seal} from "../../../backend/templates/events/web/hpke.js";
import {openSealed} from "./hpke";
import {fromBase64Url} from "./viewerKeys";
import {FIXTURE, fixturePrivateKey as privateKey} from "./test/hpkeFixture";

const INFO = new TextEncoder().encode("creators-reward/v1/enabled-donation");

describe("openSealed", () => {
	it("opens what the server sealed to the viewer's key", async () => {
		const plaintext = await openSealed(
			await privateKey(),
			fromBase64Url(FIXTURE.publicKey),
			fromBase64Url(FIXTURE.sealed),
			INFO,
		);

		const record = JSON.parse(new TextDecoder().decode(plaintext));
		expect(record.site).toBe("https://news.example");
		expect(record.enabledAtSec % 900).toBe(0);
	});

	it("opens what the events iframe seals in the browser", async () => {
		// The iframe's sender is a separate implementation; this pins it to
		// this receiver, which the case above pins to rust-hpke.
		const plaintext = new TextEncoder().encode('{"site":"https://news.example"}');

		const sealed = await seal(fromBase64Url(FIXTURE.publicKey), plaintext, INFO);

		expect(sealed.length).toBe(32 + plaintext.length + 16);
		expect(
			await openSealed(await privateKey(), fromBase64Url(FIXTURE.publicKey), sealed, INFO),
		).toEqual(plaintext);
	});

	it("rejects a record sealed for another purpose", async () => {
		await expect(
			openSealed(
				await privateKey(),
				fromBase64Url(FIXTURE.publicKey),
				fromBase64Url(FIXTURE.sealed),
				new TextEncoder().encode("creators-reward/v1/something-else"),
			),
		).rejects.toThrow();
	});

	it("rejects an altered record", async () => {
		const sealed = fromBase64Url(FIXTURE.sealed);
		sealed[sealed.length - 1]! ^= 1;

		await expect(
			openSealed(await privateKey(), fromBase64Url(FIXTURE.publicKey), sealed, INFO),
		).rejects.toThrow();
	});

	it("rejects a record too short to hold an encapsulated key", async () => {
		await expect(
			openSealed(await privateKey(), fromBase64Url(FIXTURE.publicKey), new Uint8Array(8), INFO),
		).rejects.toThrow("too short");
	});
});
