import { ensureOpaqueReady, opaque } from "./opaqueClient";
import {
	createViewerKeys,
	storeViewerPrivateKey,
	unwrapViewerPrivateKey,
} from "./viewerKeys";

const API_BASE = "/api/auth";

export class AuthApiError extends Error {
	status: number | undefined;

	constructor(message: string, status?: number) {
		super(message);
		this.name = "AuthApiError";
		this.status = status;
	}
}

export interface SessionInfo {
	email: string;
}

async function postJson(path: string, body: unknown): Promise<Response> {
	const resp = await fetch(`${API_BASE}${path}`, {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(body),
		credentials: "same-origin",
	});
	return resp;
}

export async function register(email: string, password: string): Promise<void> {
	await ensureOpaqueReady();
	const normalisedEmail = email.toLowerCase().trim();

	const { clientRegistrationState, registrationRequest } =
		opaque.client.startRegistration({ password });

	const startResp = await postJson("/register/start", {
		email: normalisedEmail,
		registrationRequest,
	});

	if (!startResp.ok) {
		if (startResp.status === 409) {
			throw new AuthApiError("Email already registered", 409);
		}
		throw new AuthApiError(`Registration start failed: ${startResp.status}`, startResp.status);
	}

	const { registrationResponse } = await startResp.json();

	const { registrationRecord, exportKey } = opaque.client.finishRegistration({
		clientRegistrationState,
		registrationResponse,
		password,
		keyStretching: "memory-constrained",
	});

	const keys = await createViewerKeys(exportKey);

	const finishResp = await postJson("/register/finish", {
		email: normalisedEmail,
		registrationRecord,
		publicKey: keys.publicKey,
		encryptedPrivateKey: keys.encryptedPrivateKey,
	});

	if (!finishResp.ok) {
		throw new AuthApiError(`Registration finish failed: ${finishResp.status}`, finishResp.status);
	}

	await storeViewerPrivateKey(keys.privateKey);
}

export async function login(email: string, password: string): Promise<void> {
	await ensureOpaqueReady();
	const normalisedEmail = email.toLowerCase().trim();

	const { clientLoginState, startLoginRequest } = opaque.client.startLogin({
		password,
	});

	const startResp = await postJson("/login/start", {
		email: normalisedEmail,
		startLoginRequest,
	});

	if (!startResp.ok) {
		throw new AuthApiError(`Login start failed: ${startResp.status}`, startResp.status);
	}

	const { loginFlowId, loginResponse } = await startResp.json();

	const loginResult = opaque.client.finishLogin({
		clientLoginState,
		loginResponse,
		password,
		keyStretching: "memory-constrained",
	});

	// CRITICAL: finishLogin returns undefined on wrong password — it does NOT throw
	if (!loginResult) {
		throw new AuthApiError("Invalid credentials", 401);
	}

	const { finishLoginRequest, exportKey } = loginResult;

	const finishResp = await postJson("/login/finish", {
		loginFlowId,
		finishLoginRequest,
	});

	if (!finishResp.ok) {
		throw new AuthApiError(`Login finish failed: ${finishResp.status}`, finishResp.status);
	}

	const { encryptedPrivateKey } = await finishResp.json();
	await storeViewerPrivateKey(await unwrapViewerPrivateKey(encryptedPrivateKey, exportKey));
}

export async function logout(): Promise<void> {
	const resp = await fetch(`${API_BASE}/logout`, {
		method: "POST",
		credentials: "same-origin",
	});
	if (!resp.ok && resp.status !== 204) {
		throw new AuthApiError(`Logout failed: ${resp.status}`, resp.status);
	}
}

export async function fetchSession(): Promise<SessionInfo | null> {
	const resp = await fetch(`${API_BASE}/session`, {
		method: "GET",
		credentials: "same-origin",
	});
	if (resp.status === 401) {
		return null;
	}
	if (!resp.ok) {
		throw new AuthApiError(`Session fetch failed: ${resp.status}`, resp.status);
	}
	return (await resp.json()) as SessionInfo;
}
