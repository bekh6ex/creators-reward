import {useLocation} from "preact-iso";
import {useState} from "preact/hooks";
import {Routes} from "../../routes";
import {useAuth} from "../../auth";
import {isValidEmail} from "../../api";
import {CreatorApiError, register} from "../../creatorApi";

const MIN_PASSWORD_LENGTH = 8;

export function Register() {
    const location = useLocation();
    const auth = useAuth();
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [confirmation, setConfirmation] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);

    function validate(): string | null {
        if (!isValidEmail(email)) {
            return "Enter a valid email address.";
        }
        if (password.length < MIN_PASSWORD_LENGTH) {
            return `Password must be at least ${MIN_PASSWORD_LENGTH} characters.`;
        }
        if (password !== confirmation) {
            return "The two passwords do not match.";
        }
        return null;
    }

    async function onSubmit(event: Event) {
        event.preventDefault();
        const problem = validate();
        setError(problem);
        if (problem !== null) {
            return;
        }
        setBusy(true);
        try {
            await register(email, password);
        } catch (failure) {
            setBusy(false);
            setError(
                failure instanceof CreatorApiError && failure.status === 409
                    ? "That email is already registered."
                    : "Could not create the account. Try again in a moment.",
            );
            return;
        }
        auth.login(email.trim().toLowerCase());
        location.route(Routes.Websites);
    }

    return (
        <div className="hero min-h-screen bg-base-200">
            <div className="hero-content w-full max-w-md">
                <div className="card w-full bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h1 className="card-title text-3xl">Create an account</h1>
                        <p className="text-base-content/70">
                            For people who publish. Readers use a different app — this one is
                            for adding your websites and getting paid.
                        </p>

                        {/* noValidate: native validation would block submit
                            before onSubmit runs. */}
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
                                        autocomplete="new-password"
                                        value={password}
                                        onInput={event => setPassword(event.currentTarget.value)}
                                    />
                                </label>
                                <label className="input w-full">
                                    <span className="label">Repeat password</span>
                                    <input
                                        type="password"
                                        autocomplete="new-password"
                                        value={confirmation}
                                        onInput={event =>
                                            setConfirmation(event.currentTarget.value)
                                        }
                                    />
                                </label>
                                <p className="label">
                                    At least {MIN_PASSWORD_LENGTH} characters.
                                </p>
                            </fieldset>

                            {error !== null && (
                                <div role="alert" className="alert alert-error mt-4">
                                    <span>{error}</span>
                                </div>
                            )}

                            <button
                                type="submit"
                                className="btn btn-primary btn-block mt-4"
                                disabled={busy}
                            >
                                Create account
                            </button>
                        </form>

                        <p className="label mt-2">
                            Already have an account?{" "}
                            <a className="link" href={Routes.Login}>Log in</a>
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
}
