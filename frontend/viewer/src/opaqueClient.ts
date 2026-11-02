import * as opaque from "@serenity-kit/opaque";

let readyPromise: Promise<void> | null = null;

export async function ensureOpaqueReady(): Promise<void> {
	if (!readyPromise) {
		readyPromise = opaque.ready;
	}
	return readyPromise;
}

export { opaque };
