import {openSealed} from "./hpke";
import type {EnabledDonation} from "./types";
import {fromBase64Url, loadViewerPrivateKey} from "./viewerKeys";

// Same `info` the events iframe seals with (backend/templates/events/web/iframe.html).
const API = "/api/events/web/enabled-donations";
const INFO = new TextEncoder().encode("creators-reward/v1/enabled-donation");

export class EnabledDonationsError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "EnabledDonationsError";
	}
}

interface SealedEnabledDonations {
	publicKey: string;
	items: {id: string; sealed: string}[];
}

interface EnabledDonationRecord {
	site: string;
	enabledAtSec: number;
}

function isRecord(value: unknown): value is EnabledDonationRecord {
	return (
		typeof value === "object" &&
		value !== null &&
		typeof (value as EnabledDonationRecord).site === "string" &&
		typeof (value as EnabledDonationRecord).enabledAtSec === "number"
	);
}

// The events iframe seals each record in the browser to this viewer's public
// key, so the server only ever holds ciphertext. It is opened here with the
// private key that login left in IndexedDB.
export async function fetchEnabledDonations(): Promise<EnabledDonation[]> {
	const resp = await fetch(API, {credentials: "same-origin"});
	if (!resp.ok) {
		throw new EnabledDonationsError(`Loading enabled donations failed: ${resp.status}`);
	}
	const body = (await resp.json()) as SealedEnabledDonations;

	const privateKey = await loadViewerPrivateKey();
	if (!privateKey) {
		throw new EnabledDonationsError("Your key is not unlocked in this browser. Log in again.");
	}
	const publicKey = fromBase64Url(body.publicKey);
	const decoder = new TextDecoder();
	const records = await Promise.all(
		body.items.map(async ({id, sealed}) => {
			const plaintext = await openSealed(privateKey, publicKey, fromBase64Url(sealed), INFO);
			const record: unknown = JSON.parse(decoder.decode(plaintext));
			if (!isRecord(record)) throw new EnabledDonationsError("Malformed enabled donation");
			return {id, ...record};
		}),
	);

	// The server cannot see what it stores, so it cannot deduplicate. Records
	// arrive oldest first: the first per site says since when it is on, and
	// every id is kept so that disabling removes them all.
	const bySite = new Map<string, EnabledDonation>();
	for (const {id, site, enabledAtSec} of records) {
		const existing = bySite.get(site);
		if (existing) {
			existing.recordIds.push(id);
		} else {
			bySite.set(site, {
				site,
				enabledAt: new Date(enabledAtSec * 1000).toISOString(),
				recordIds: [id],
			});
		}
	}
	return [...bySite.values()];
}

// Deletes the records, which also stops collection on that site: events must
// carry a record id that still exists. The server learns which random ids
// went, never which site.
export async function disableDonations(donation: EnabledDonation): Promise<void> {
	const responses = await Promise.all(
		donation.recordIds.map((id) =>
			fetch(`${API}/${encodeURIComponent(id)}`, {method: "DELETE", credentials: "same-origin"}),
		),
	);
	const failed = responses.find((resp) => !resp.ok);
	if (failed) {
		throw new EnabledDonationsError(`Disabling donations failed: ${failed.status}`);
	}
}
