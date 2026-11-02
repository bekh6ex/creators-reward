import {useCallback} from "preact/hooks";
import {useLocation} from "preact-iso";
import {useState} from "preact/hooks";
import {Routes} from "../../routes";
import {useAuth} from "../../auth";
import {register as registerApi, AuthApiError} from "../../authApi";

const MIN_PASSWORD_LENGTH = 8;

export function Register() {
    const location = useLocation();
    const auth = useAuth();
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [confirmation, setConfirmation] = useState("");
    const [isPending, setIsPending] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const validate = useCallback((): string | null => {
        if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email)) {
            return "Enter a valid email address.";
        }
        if (password.length < MIN_PASSWORD_LENGTH) {
            return `Password must be at least ${MIN_PASSWORD_LENGTH} characters.`;
        }
        if (password !== confirmation) {
            return "The two passwords do not match.";
        }
        return null;
    }, [email, password, confirmation]);

    const onSubmit = useCallback(
        async (event: Event) => {
            event.preventDefault();
            const validationError = validate();
            if (validationError) {
                setError(validationError);
                return;
            }
            setIsPending(true);
            setError(null);
            try {
                await registerApi(email, password);
                auth.login(email);
                location.route(Routes.Dashboard);
            } catch (err) {
                if (err instanceof AuthApiError && err.status === 409) {
                    setError(
                        "This email is already registered. Please log in instead.",
                    );
                } else if (err instanceof AuthApiError && err.status === 422) {
                    setError("Invalid input. Please check your details.");
                } else {
                    setError("Registration failed. Please try again.");
                }
            } finally {
                setIsPending(false);
            }
        },
        [email, password, validate, auth, location],
    );

    return (
        <div className="hero min-h-screen bg-base-200">
            <div className="hero-content w-full max-w-md">
                <div className="card w-full bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h1 className="card-title text-3xl">Create an account</h1>
                        <p className="text-base-content/70">
                            We only ask for an email and a password. Nothing else.
                        </p>

                        <div role="alert" className="alert alert-warning mt-2">
                            <span>
                                Your password unlocks your reading history. We cannot reset
                                it — if you forget it, that history is unreadable forever.
                            </span>
                        </div>

                        <form onSubmit={onSubmit}>
                            <fieldset className="fieldset">
                                <label className="input w-full">
                                    <span className="label">Email</span>
                                    <input
                                        type="email"
                                        autocomplete="username"
                                        value={email}
                                        disabled={isPending}
                                        onInput={event => setEmail(event.currentTarget.value)}
                                    />
                                </label>
                                <label className="input w-full">
                                    <span className="label">Password</span>
                                    <input
                                        type="password"
                                        autocomplete="new-password"
                                        value={password}
                                        disabled={isPending}
                                        onInput={event => setPassword(event.currentTarget.value)}
                                    />
                                </label>
                                <label className="input w-full">
                                    <span className="label">Repeat password</span>
                                    <input
                                        type="password"
                                        autocomplete="new-password"
                                        value={confirmation}
                                        disabled={isPending}
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
                                disabled={isPending}
                            >
                                {isPending ? "Creating account…" : "Create account"}
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
