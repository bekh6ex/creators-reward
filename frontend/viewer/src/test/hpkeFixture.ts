import {fromBase64Url} from "../viewerKeys";

// Sealed by the server's rust-hpke; regenerate with
//   cargo test -p web_viewer_events print_browser_fixture -- --ignored --nocapture
export const FIXTURE = {
	privateKeyPkcs8: "MC4CAQAwBQYDK2VuBCIEIIDkINkOhzTATBvE8f_u5zLkabRrBf4LCzKax2ShyCZL",
	publicKey: "b9VDvqz7QvPpxd4da04TpfmWRRJqGgKKny_5bW6Gn3A",
	sealed:
		"qqEWtwvxuATUeRnw29M7VWkrFimUw8JiX3NA0ez0mE_k8ikcZfa-FVIgcGSH00mRsfjyB3WQijE8r80abYuMbhvgWJ_NvXPfG5dQxAUVR_dJrxmQqT9RMIE9-PwVe1iJEzUmT6CEzPbSeXbPSV2fWdl8Gi6-7SCUwfyJK2ShXHW7x6k-QzBF5quoEe1_VsWfTtCH8EcXjc7sxHIWeFQERkq663-oBQFcwFf0eQraTAz4YvfkGMmARP9CzV941qDXCcY4BGumIIX2c5frEAZZvVSnzWJQlhi4L5KM8k1fCNRLsmXtiX6NLLECDekcBnjZ0qC2AGFBqz4AQ2GN66cmtmUlIsq8c3vJK6WbPBqMjhYlbwIB6V_JB7_Dx4IOKaRQaptxYPpdiVcKR4vmTLy9nw",
};
export function fixturePrivateKey(): Promise<CryptoKey> {
	return crypto.subtle.importKey(
		"pkcs8",
		fromBase64Url(FIXTURE.privateKeyPkcs8),
		{name: "X25519"},
		false,
		["deriveBits"],
	);
}
