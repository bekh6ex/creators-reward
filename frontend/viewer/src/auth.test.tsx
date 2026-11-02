import {afterEach, describe, expect, it, vi} from "vitest";
import {fireEvent, render, waitFor} from "@testing-library/preact";
import {AuthProvider, useAuth} from "./auth";
import type {ComponentChildren} from "preact";

vi.mock("./authApi", () => ({
	fetchSession: vi.fn(),
	logout: vi.fn(),
}));

vi.mock("./viewerKeys", () => ({
	loadViewerPrivateKey: vi.fn(),
	clearViewerPrivateKey: vi.fn(),
}));

import {fetchSession, logout as logoutApi} from "./authApi";
import {clearViewerPrivateKey, loadViewerPrivateKey} from "./viewerKeys";

const storedKey = await crypto.subtle.generateKey({name: "AES-GCM", length: 256}, false, [
	"encrypt",
]);

function TestConsumer() {
	const auth = useAuth();
	return (
		<div>
			<span data-testid="email">{auth.email ?? "null"}</span>
			<span data-testid="isLoggedIn">{auth.isLoggedIn ? "true" : "false"}</span>
			<button type="button" onClick={() => auth.login("bob@example.com")}>
				Login
			</button>
			<button type="button" onClick={() => auth.logout()}>
				Logout
			</button>
		</div>
	);
}

function renderAuthProvider(ui: ComponentChildren, initialEmail?: string) {
	const props = initialEmail !== undefined ? {initialEmail} : {};
	return render(<AuthProvider {...props}>{ui}</AuthProvider>);
}

describe("AuthProvider", () => {
	afterEach(() => {
		vi.mocked(fetchSession).mockReset();
		vi.mocked(logoutApi).mockReset();
		vi.mocked(loadViewerPrivateKey).mockReset();
		vi.mocked(clearViewerPrivateKey).mockReset();
	});

	it("boots with live session", async () => {
		vi.mocked(fetchSession).mockResolvedValue({email: "alice@example.com"});
		vi.mocked(loadViewerPrivateKey).mockResolvedValue(storedKey);

		const {getByTestId} = renderAuthProvider(<TestConsumer />);

		await waitFor(() => {
			expect(getByTestId("isLoggedIn").textContent).toBe("true");
		});
		expect(getByTestId("email").textContent).toBe("alice@example.com");
	});

	it("boots with no session", async () => {
		vi.mocked(fetchSession).mockResolvedValue(null);

		const {getByTestId} = renderAuthProvider(<TestConsumer />);

		await waitFor(() => {
			expect(getByTestId("isLoggedIn").textContent).toBe("false");
		});
		expect(getByTestId("email").textContent).toBe("null");
		expect(clearViewerPrivateKey).toHaveBeenCalledTimes(1);
	});

	it("boots with network error", async () => {
		vi.mocked(fetchSession).mockRejectedValue(new Error("network error"));

		const {getByTestId} = renderAuthProvider(<TestConsumer />);

		await waitFor(() => {
			expect(getByTestId("isLoggedIn").textContent).toBe("false");
		});
		expect(getByTestId("email").textContent).toBe("null");
	});

	it("ends a live session whose private key is not stored in this browser", async () => {
		vi.mocked(fetchSession).mockResolvedValue({email: "alice@example.com"});
		vi.mocked(loadViewerPrivateKey).mockResolvedValue(null);
		vi.mocked(logoutApi).mockResolvedValue(undefined);

		const {getByTestId} = renderAuthProvider(<TestConsumer />);

		await waitFor(() => {
			expect(logoutApi).toHaveBeenCalledTimes(1);
		});
		expect(getByTestId("isLoggedIn").textContent).toBe("false");
	});

	it("logout calls API, forgets the private key and clears state", async () => {
		vi.mocked(fetchSession).mockResolvedValue({email: "alice@example.com"});
		vi.mocked(loadViewerPrivateKey).mockResolvedValue(storedKey);
		vi.mocked(logoutApi).mockResolvedValue(undefined);
		vi.mocked(clearViewerPrivateKey).mockResolvedValue(undefined);

		const {getByTestId, getByRole} = renderAuthProvider(<TestConsumer />);

		await waitFor(() => {
			expect(getByTestId("isLoggedIn").textContent).toBe("true");
		});

		fireEvent.click(getByRole("button", {name: "Logout"}));

		await waitFor(() => {
			expect(getByTestId("isLoggedIn").textContent).toBe("false");
		});
		expect(getByTestId("email").textContent).toBe("null");
		expect(logoutApi).toHaveBeenCalledTimes(1);
		expect(clearViewerPrivateKey).toHaveBeenCalledTimes(1);
	});

	it("login sets email", async () => {
		vi.mocked(fetchSession).mockResolvedValue(null);

		const {getByTestId, getByRole} = renderAuthProvider(<TestConsumer />);

		await waitFor(() => {
			expect(getByTestId("isLoggedIn").textContent).toBe("false");
		});

		fireEvent.click(getByRole("button", {name: "Login"}));

		await waitFor(() => {
			expect(getByTestId("isLoggedIn").textContent).toBe("true");
		});
		expect(getByTestId("email").textContent).toBe("bob@example.com");
	});
});
