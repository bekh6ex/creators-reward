// @vitest-environment node
import { describe, it, expect } from "vitest";
import { ensureOpaqueReady, opaque } from "./opaqueClient";

describe("opaqueClient", () => {
	it("initialises and can start registration", async () => {
		await ensureOpaqueReady();
		const { registrationRequest } = opaque.client.startRegistration({
			password: "test-password",
		});
		expect(registrationRequest).toBeTruthy();
		expect(typeof registrationRequest).toBe("string");
		expect(registrationRequest.length).toBeGreaterThan(0);
	});
});
