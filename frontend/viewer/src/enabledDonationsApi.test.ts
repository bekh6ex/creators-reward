// @vitest-environment node
import {indexedDB as fakeIndexedDB} from "fake-indexeddb";
import {afterEach, beforeAll, describe, expect, it, vi} from "vitest";
import {
	disableDonations,
	EnabledDonationsError,
	fetchEnabledDonations,
} from "./enabledDonationsApi";
import {FIXTURE, fixturePrivateKey} from "./test/hpkeFixture";
import {clearViewerPrivateKey, storeViewerPrivateKey} from "./viewerKeys";

function respondWith(status: number, body?: unknown) {
	vi.stubGlobal(
		"fetch",
		vi.fn(async () => new Response(body === undefined ? null : JSON.stringify(body), {status})),
	);
}

describe("fetchEnabledDonations", () => {
	beforeAll(() => {
		vi.stubGlobal("indexedDB", fakeIndexedDB);
	});

	afterEach(async () => {
		await clearViewerPrivateKey();
		vi.unstubAllGlobals();
		vi.stubGlobal("indexedDB", fakeIndexedDB);
	});

	it("opens every sealed record and groups the record ids per site", async () => {
		await storeViewerPrivateKey(await fixturePrivateKey());
		respondWith(200, {
			publicKey: FIXTURE.publicKey,
			items: [
				{id: "first", sealed: FIXTURE.sealed},
				{id: "second", sealed: FIXTURE.sealed},
			],
		});

		const donations = await fetchEnabledDonations();

		expect(donations).toHaveLength(1);
		expect(donations[0]!.site).toBe("https://news.example");
		expect(donations[0]!.recordIds).toEqual(["first", "second"]);
		expect(new Date(donations[0]!.enabledAt).getUTCMinutes() % 15).toBe(0);
		expect(fetch).toHaveBeenCalledWith("/api/events/web/enabled-donations", {
			credentials: "same-origin",
		});
	});

	it("fails with the status when the events service refuses", async () => {
		respondWith(401);

		await expect(fetchEnabledDonations()).rejects.toThrow(
			new EnabledDonationsError("Loading enabled donations failed: 401"),
		);
	});

	it("asks to log in again when the private key is not in this browser", async () => {
		respondWith(200, {publicKey: FIXTURE.publicKey, items: [{id: "x", sealed: FIXTURE.sealed}]});

		await expect(fetchEnabledDonations()).rejects.toThrow("Log in again.");
	});

	it("deletes every record of the site", async () => {
		respondWith(204);

		await disableDonations({site: "https://news.example", enabledAt: "", recordIds: ["a", "b/c"]});

		expect(fetch).toHaveBeenCalledWith("/api/events/web/enabled-donations/a", {
			method: "DELETE",
			credentials: "same-origin",
		});
		expect(fetch).toHaveBeenCalledWith("/api/events/web/enabled-donations/b%2Fc", {
			method: "DELETE",
			credentials: "same-origin",
		});
	});

	it("fails when the events service refuses a delete", async () => {
		vi.stubGlobal("fetch", vi.fn(async () => new Response(null, {status: 401})));

		await expect(
			disableDonations({site: "https://news.example", enabledAt: "", recordIds: ["a"]}),
		).rejects.toThrow("Disabling donations failed: 401");
	});
});
