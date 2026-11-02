import {useLocation, useRoute} from "preact-iso";
import {useState} from "preact/hooks";
import {Routes} from "../../routes";

const MIN_PASSWORD_LENGTH = 8;
// Stand-in for a token the backend would reject. Everything else is accepted so
// the happy path stays walkable without a mail server.
const EXPIRED_TOKEN = "expired";

export function ResetPassword(props: {token: string}) {
    const location = useLocation();
    const [password, setPassword] = useState("");
    const [confirmation, setConfirmation] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [done, setDone] = useState(false);

    const tokenIsValid = props.token !== "" && props.token !== EXPIRED_TOKEN;

    function onSubmit(event: Event) {
        event.preventDefault();
        if (password.length < MIN_PASSWORD_LENGTH) {
            setError(`Password must be at least ${MIN_PASSWORD_LENGTH} characters.`);
            return;
        }
        if (password !== confirmation) {
            setError("The two passwords do not match.");
            return;
        }
        setError(null);
        setDone(true);
    }

    return (
        <div className="hero min-h-screen bg-base-200">
            <div className="hero-content w-full max-w-md">
                <div className="card w-full bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h1 className="card-title text-3xl">Choose a new password</h1>

                        {!tokenIsValid && (
                            <>
                                <div role="alert" className="alert alert-error">
                                    <span>
                                        This reset link has expired. Ask for a new one.
                                    </span>
                                </div>
                                <p className="label">
                                    <a className="link" href={Routes.ForgotPassword}>
                                        Send another reset link
                                    </a>
                                </p>
                            </>
                        )}

                        {tokenIsValid && done && (
                            <>
                                <div role="alert" className="alert alert-success">
                                    <span>Your password has been changed.</span>
                                </div>
                                <button
                                    type="button"
                                    className="btn btn-primary btn-block mt-4"
                                    onClick={() => location.route(Routes.Login)}
                                >
                                    Go to login
                                </button>
                            </>
                        )}

                        {tokenIsValid && !done && (
                            <form onSubmit={onSubmit} noValidate>
                                <fieldset className="fieldset">
                                    <label className="input w-full">
                                        <span className="label">New password</span>
                                        <input
                                            type="password"
                                            autocomplete="new-password"
                                            value={password}
                                            onInput={event =>
                                                setPassword(event.currentTarget.value)
                                            }
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
                                >
                                    Change password
                                </button>
                            </form>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}

// preact-iso exposes route params through useRoute() rather than as props, so
// the page stays presentational and this adapter is what the Router mounts.
export function ResetPasswordRoute() {
    const {params} = useRoute();
    return <ResetPassword token={params["token"] ?? ""}/>;
}
