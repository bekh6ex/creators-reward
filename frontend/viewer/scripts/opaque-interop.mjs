import { spawn } from "child_process";
import { fileURLToPath } from "url";
import * as opaque from "@serenity-kit/opaque";

const BACKEND_PORT = 18734;
const BASE_URL = `http://127.0.0.1:${BACKEND_PORT}`;

async function main() {
  await opaque.ready;

  const rustProcess = spawn("cargo", ["run"], {
    cwd: fileURLToPath(new URL("../../../backend", import.meta.url)),
    env: {
      ...process.env,
      CR_BACKEND_PORT: String(BACKEND_PORT),
      CR_BIND_ADDR: "127.0.0.1",
    },
    detached: true,
    stdio: "pipe",
  });

  function cleanup() {
    if (rustProcess.pid && !rustProcess.killed) {
      try {
        process.kill(-rustProcess.pid, "SIGTERM");
      } catch {
        try {
          rustProcess.kill("SIGTERM");
        } catch {
          // ignore
        }
      }
    }
  }

  process.on("exit", cleanup);

  try {
    // Wait for server to be ready by polling /api/auth/session
    console.log("Waiting for backend to be ready...");
    const ready = await waitForServer(`${BASE_URL}/api/auth/session`, 15000);
    if (!ready) {
      throw new Error("Backend did not become ready in time");
    }
    console.log("Backend is ready");

    const testEmail = "alice@example.com";
    const password = "test-password-123";

    // === Registration Flow ===
    console.log("Starting registration...");

    const { clientRegistrationState, registrationRequest } =
      opaque.client.startRegistration({ password });

    const regStartResp = await fetch(`${BASE_URL}/api/auth/register/start`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: testEmail,
        registrationRequest,
      }),
    });

    if (!regStartResp.ok) {
      throw new Error(`register/start failed: ${regStartResp.status}`);
    }

    const { registrationResponse } = await regStartResp.json();

    const { registrationRecord } = opaque.client.finishRegistration({
      clientRegistrationState,
      registrationResponse,
      password,
      keyStretching: "memory-constrained",
    });

    const regFinishResp = await fetch(`${BASE_URL}/api/auth/register/finish`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: testEmail,
        registrationRecord,
      }),
    });

    if (!regFinishResp.ok) {
      throw new Error(`register/finish failed: ${regFinishResp.status}`);
    }

    console.log("Registration complete");

    // === Login Flow ===
    console.log("Starting login...");

    const { clientLoginState, startLoginRequest } = opaque.client.startLogin({
      password,
    });

    const loginStartResp = await fetch(`${BASE_URL}/api/auth/login/start`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: testEmail,
        startLoginRequest,
      }),
    });

    if (!loginStartResp.ok) {
      throw new Error(`login/start failed: ${loginStartResp.status}`);
    }

    const { loginFlowId, loginResponse } = await loginStartResp.json();

    const loginResult = opaque.client.finishLogin({
      clientLoginState,
      loginResponse,
      password,
      keyStretching: "memory-constrained",
    });

    if (!loginResult) {
      throw new Error("Client finishLogin returned undefined for known user");
    }

    const { finishLoginRequest } = loginResult;

    const loginFinishResp = await fetch(`${BASE_URL}/api/auth/login/finish`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        loginFlowId,
        finishLoginRequest,
      }),
    });

    if (!loginFinishResp.ok) {
      throw new Error(`login/finish failed: ${loginFinishResp.status}`);
    }

    // Extract session cookie from login/finish response
    const sessionCookie = extractCookie(loginFinishResp.headers.getSetCookie?.() ?? [], "cr_session");
    if (!sessionCookie) {
      throw new Error("cr_session cookie missing from login/finish response");
    }

    // Verify session via GET /api/auth/session
    const sessionResp = await fetch(`${BASE_URL}/api/auth/session`, {
      headers: { Cookie: `cr_session=${sessionCookie}` },
    });

    if (!sessionResp.ok) {
      throw new Error(`session check failed: ${sessionResp.status}`);
    }

    const sessionBody = await sessionResp.json();
    if (sessionBody.email !== testEmail) {
      throw new Error(
        `Session email mismatch: expected ${testEmail}, got ${sessionBody.email}`
      );
    }

    console.log("Login complete, session verified");

    // === Unknown Email Fake Path ===
    console.log("Testing unknown email fake path...");

    const unknownEmail = "unknown@example.com";

    const {
      clientLoginState: unknownClientLoginState,
      startLoginRequest: unknownStartLoginRequest,
    } = opaque.client.startLogin({ password });

    const unknownLoginStartResp = await fetch(`${BASE_URL}/api/auth/login/start`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: unknownEmail,
        unknownStartLoginRequest,
      }),
    });

    if (!unknownLoginStartResp.ok) {
      throw new Error(`unknown login/start failed: ${unknownLoginStartResp.status}`);
    }

    const { loginResponse: unknownLoginResponse } = await unknownLoginStartResp.json();

    const unknownLoginResult = opaque.client.finishLogin({
      clientLoginState: unknownClientLoginState,
      loginResponse: unknownLoginResponse,
      password,
      keyStretching: "memory-constrained",
    });

    if (unknownLoginResult !== undefined) {
      throw new Error(
        `Unknown email fake path should return undefined, got: ${JSON.stringify(unknownLoginResult)}`
      );
    }

    console.log("Unknown email fake path correctly returned undefined");

    console.log("INTEROP OK");
    process.exitCode = 0;
  } catch (err) {
    console.error(err);
    process.exitCode = 1;
  } finally {
    cleanup();
  }
}

/**
 * @param {string[]} setCookies
 * @param {string} name
 * @returns {string | undefined}
 */
function extractCookie(setCookies, name) {
  const prefix = `${name}=`;
  for (const header of setCookies) {
    if (header.startsWith(prefix)) {
      const end = header.indexOf(";");
      return end === -1 ? header.slice(prefix.length) : header.slice(prefix.length, end);
    }
  }
  return undefined;
}

/**
 * @param {string} url
 * @param {number} timeoutMs
 * @returns {Promise<boolean>}
 */
async function waitForServer(url, timeoutMs) {
  const deadline = Date.now() + timeoutMs;

  async function tryConnect() {
    try {
      const resp = await fetch(url, { signal: AbortSignal.timeout(2000) });
      // Any response (even 401) means the server is up
      if (resp.status === 401 || resp.status === 200) {
        return true;
      }
    } catch {
      // not ready yet
    }
    if (Date.now() >= deadline) {
      return false;
    }
    await new Promise((r) => setTimeout(r, 300));
    return tryConnect();
  }

  return tryConnect();
}

main();
