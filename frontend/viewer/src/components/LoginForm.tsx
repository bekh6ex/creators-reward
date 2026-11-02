import {useLocation} from "preact-iso";
import {useCallback, useState} from "preact/hooks";
import {Routes} from "../routes";
import {useAuth} from "../auth";
import {login as loginApi, AuthApiError} from "../authApi";

export function LoginForm() {
    const location = useLocation();
    const auth = useAuth();
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [isPending, setIsPending] = useState(false);

    const onSubmit = useCallback(
        async (e: Event) => {
            e.preventDefault();
            if (!email || !password) {
                setError("Please fill in all fields");
                return;
            }
            setIsPending(true);
            setError(null);
            try {
                await loginApi(email, password);
                auth.login(email);
                location.route(Routes.Dashboard);
            } catch (err) {
                if (err instanceof AuthApiError && err.status === 401) {
                    setError("Invalid email or password");
                } else {
                    setError("Network error. Please try again.");
                }
            } finally {
                setIsPending(false);
            }
        },
        [email, password, auth, location],
    );

    return (
        <form onSubmit={onSubmit}>
            <fieldset className="fieldset">
                <label className="input w-full">
                    <span className="label">Email</span>
                    <input
                        type="email"
                        autocomplete="username"
                        value={email}
                        onInput={event => setEmail(event.currentTarget.value)}
                    />
                </label>
                <label className="input w-full">
                    <span className="label">Password</span>
                    <input
                        type="password"
                        autocomplete="current-password"
                        value={password}
                        onInput={event => setPassword(event.currentTarget.value)}
                    />
                </label>
            </fieldset>
            {error !== null && (
                <div role="alert" className="alert alert-error mt-4">
                    <span>{error}</span>
                </div>
            )}
            <button type="submit" className="btn btn-primary btn-block mt-4" disabled={isPending}>
                {isPending ? "Signing in…" : "Sign in"}
            </button>
        </form>
    );
}
