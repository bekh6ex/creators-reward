import {useLocation} from "preact-iso";
import {useState} from "preact/hooks";
import {Routes} from "../routes";
import {useAuth} from "../auth";
import {CreatorApiError, login} from "../creatorApi";

export function LoginForm() {
    const location = useLocation();
    const auth = useAuth();
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);

    async function onSubmit(event: Event) {
        event.preventDefault();
        if (email.trim() === "" || password === "") {
            setError("Enter your email and password.");
            return;
        }
        setError(null);
        setBusy(true);
        try {
            await login(email, password);
        } catch (failure) {
            setBusy(false);
            setError(
                failure instanceof CreatorApiError && failure.status === 401
                    ? "Wrong email or password."
                    : "Could not log in. Try again in a moment.",
            );
            return;
        }
        auth.login(email.trim().toLowerCase());
        location.route(Routes.Websites);
    }

    return (
        // noValidate: native validation would block submit before onSubmit runs.
        <form onSubmit={onSubmit} noValidate>
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
            <button type="submit" className="btn btn-primary btn-block mt-4" disabled={busy}>
                Log in
            </button>
            <p className="label mt-2">
                <a className="link" href={Routes.ForgotPassword}>Forgot your password?</a>
            </p>
        </form>
    );
}
