import type {Website} from "./types";

// The creator service (backend/services/creator), reached as /api on this portal's
// own host. Accounts and websites are real; payouts, statistics, profile and
// password recovery are still faked in api.ts.

const API_BASE = "/api";

export class CreatorApiError extends Error {
    status: number;

    constructor(message: string, status: number) {
        super(message);
        this.name = "CreatorApiError";
        this.status = status;
    }
}

async function request(path: string, init: RequestInit = {}): Promise<Response> {
    return await fetch(`${API_BASE}${path}`, {...init, credentials: "same-origin"});
}

async function postJson(path: string, body: unknown): Promise<Response> {
    return await request(path, {
        method: "POST",
        headers: {"Content-Type": "application/json"},
        body: JSON.stringify(body),
    });
}

function failure(what: string, response: Response): CreatorApiError {
    return new CreatorApiError(`${what} failed: ${response.status}`, response.status);
}

/** Creates the account and logs it in. 409: email taken. */
export async function register(email: string, password: string): Promise<void> {
    const response = await postJson("/auth/register", {email, password});
    if (!response.ok) throw failure("Registration", response);
}

/** 401: wrong email or password. */
export async function login(email: string, password: string): Promise<void> {
    const response = await postJson("/auth/login", {email, password});
    if (!response.ok) throw failure("Login", response);
}

export async function logout(): Promise<void> {
    const response = await request("/auth/logout", {method: "POST"});
    if (!response.ok) throw failure("Logout", response);
}

/** The logged-in creator's email, or null without a live session. */
export async function fetchSession(): Promise<string | null> {
    const response = await request("/auth/session");
    if (response.status === 401) return null;
    if (!response.ok) throw failure("Session", response);
    const {email} = (await response.json()) as {email: string};
    return email;
}

export async function listWebsites(): Promise<Website[]> {
    const response = await request("/websites");
    if (!response.ok) throw failure("Listing websites", response);
    return (await response.json()) as Website[];
}

/** 409: some account already added this domain. */
export async function addWebsite(domain: string, title: string): Promise<Website> {
    const response = await postJson("/websites", {domain, title});
    if (!response.ok) throw failure("Adding the website", response);
    return (await response.json()) as Website;
}
