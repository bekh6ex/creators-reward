import {describe, expect, it, vi, beforeEach, afterEach} from "vitest";
import {register, login, logout, fetchSession, AuthApiError} from "./authApi";
import {opaque} from "./opaqueClient";
import {createViewerKeys, storeViewerPrivateKey, unwrapViewerPrivateKey} from "./viewerKeys";

vi.mock("./opaqueClient", () => ({
	ensureOpaqueReady: vi.fn().mockResolvedValue(undefined),
	opaque: {
		client: {
			startRegistration: vi.fn(() => ({ clientRegistrationState: "state", registrationRequest: "req" })),
			finishRegistration: vi.fn(() => ({ registrationRecord: "rec", exportKey: "export123" })),
			startLogin: vi.fn(() => ({ clientLoginState: "state", startLoginRequest: "req" })),
			finishLogin: vi.fn(() => ({
				finishLoginRequest: "fin",
				sessionKey: "session-key",
				exportKey: "login-export-key",
			})),
		},
	},
}));

const {privateKey} = vi.hoisted(() => ({privateKey: Symbol("viewer private key")}));

vi.mock("./viewerKeys", () => ({
	createViewerKeys: vi.fn(async () => ({
		publicKey: "pub",
		encryptedPrivateKey: "sealed",
		privateKey,
	})),
	unwrapViewerPrivateKey: vi.fn(async () => privateKey),
	storeViewerPrivateKey: vi.fn(async () => undefined),
}));


describe("authApi", () => {
	let fetchMock: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		fetchMock = vi.fn();
		vi.stubGlobal("fetch", fetchMock);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		vi.mocked(opaque.client.startRegistration).mockClear();
		vi.mocked(opaque.client.finishRegistration).mockClear();
		vi.mocked(opaque.client.startLogin).mockClear();
		vi.mocked(opaque.client.finishLogin).mockClear();
		vi.mocked(createViewerKeys).mockClear();
		vi.mocked(unwrapViewerPrivateKey).mockClear();
		vi.mocked(storeViewerPrivateKey).mockClear();
	});

	describe("register", () => {
		it("sends key material sealed under the export key and stores the private key", async () => {
			fetchMock
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({ registrationResponse: "resp" }),
				})
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({}),
				});

			await register("Test@Example.COM", "password");

			expect(createViewerKeys).toHaveBeenCalledWith("export123");
			expect(fetchMock).toHaveBeenNthCalledWith(
				2,
				"/api/auth/register/finish",
				expect.objectContaining({
					body: JSON.stringify({
						email: "test@example.com",
						registrationRecord: "rec",
						publicKey: "pub",
						encryptedPrivateKey: "sealed",
					}),
				}),
			);
			expect(storeViewerPrivateKey).toHaveBeenCalledWith(privateKey);
		});

		it("throws AuthApiError with status 409 when email already registered", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: false,
				status: 409,
				json: async () => ({}),
			});

			await expect(register("test@example.com", "password")).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 409 &&
					err.message === "Email already registered",
			);
		});

		it("throws AuthApiError on other start error status", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: false,
				status: 500,
				json: async () => ({}),
			});

			await expect(register("test@example.com", "password")).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 500 &&
					err.message === "Registration start failed: 500",
			);
		});

		it("throws AuthApiError on finish failure", async () => {
			fetchMock
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({ registrationResponse: "resp" }),
				})
				.mockResolvedValueOnce({
					ok: false,
					status: 500,
					json: async () => ({}),
				});

			await expect(register("test@example.com", "password")).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 500 &&
					err.message === "Registration finish failed: 500",
			);
			expect(storeViewerPrivateKey).not.toHaveBeenCalled();
		});

		it("propagates network error from fetch", async () => {
			fetchMock.mockRejectedValueOnce(new Error("network down"));

			await expect(register("test@example.com", "password")).rejects.toThrow("network down");
		});
	});

	describe("login", () => {
		it("unseals the private key with the export key, not the session key, and stores it", async () => {
			fetchMock
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({ loginFlowId: "flow1", loginResponse: "resp" }),
				})
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({ encryptedPrivateKey: "sealed" }),
				});

			await login("test@example.com", "password");

			expect(unwrapViewerPrivateKey).toHaveBeenCalledWith("sealed", "login-export-key");
			expect(storeViewerPrivateKey).toHaveBeenCalledWith(privateKey);
		});

		it("throws AuthApiError with status 401 on wrong password without calling finish", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: true,
				status: 200,
				json: async () => ({ loginFlowId: "flow1", loginResponse: "resp" }),
			});

			vi.mocked(opaque.client.finishLogin).mockReturnValueOnce(undefined);

			await expect(login("test@example.com", "password")).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 401 &&
					err.message === "Invalid credentials",
			);

			expect(fetchMock).toHaveBeenCalledTimes(1);
		});

		it("throws AuthApiError on 401 at finish", async () => {
			fetchMock
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({ loginFlowId: "flow1", loginResponse: "resp" }),
				})
				.mockResolvedValueOnce({
					ok: false,
					status: 401,
					json: async () => ({}),
				});

			await expect(login("test@example.com", "password")).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 401 &&
					err.message === "Login finish failed: 401",
			);
		});

		it("throws AuthApiError on non-2xx at start", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: false,
				status: 500,
				json: async () => ({}),
			});

			await expect(login("test@example.com", "password")).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 500 &&
					err.message === "Login start failed: 500",
			);
		});
	});

	describe("logout", () => {
		it("succeeds on 204 without throwing", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: true,
				status: 204,
			});

			await expect(logout()).resolves.toBeUndefined();
		});

		it("throws AuthApiError on failure", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: false,
				status: 500,
			});

			await expect(logout()).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 500 &&
					err.message === "Logout failed: 500",
			);
		});
	});

	describe("fetchSession", () => {
		it("returns session info on 200", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: true,
				status: 200,
				json: async () => ({ email: "alice@example.com" }),
			});

			const session = await fetchSession();
			expect(session).toEqual({ email: "alice@example.com" });
		});

		it("returns null on 401", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: false,
				status: 401,
			});

			const session = await fetchSession();
			expect(session).toBeNull();
		});

		it("throws AuthApiError on other error status", async () => {
			fetchMock.mockResolvedValueOnce({
				ok: false,
				status: 500,
			});

			await expect(fetchSession()).rejects.toSatisfy(
				(err: unknown) =>
					err instanceof AuthApiError &&
					err.status === 500 &&
					err.message === "Session fetch failed: 500",
			);
		});
	});

	describe("exportKey non-persistence", () => {
		it("does not store exportKey in localStorage, sessionStorage, or cookies after login", async () => {
			fetchMock
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({ loginFlowId: "flow1", loginResponse: "resp" }),
				})
				.mockResolvedValueOnce({
					ok: true,
					status: 200,
					json: async () => ({ encryptedPrivateKey: "sealed" }),
				});

			await login("test@example.com", "password");

			expect(localStorage.length).toBe(0);
			expect(sessionStorage.length).toBe(0);
			expect(document.cookie).not.toContain("login-export-key");
		});
	});
});
