import {useState} from "preact/hooks";
import {Routes, resetPasswordRoute} from "../../routes";
import {isValidEmail} from "../../api";

// The link a real backend would email. Hard-coded because nothing here sends
// mail; the page surfaces it so the reset flow can be walked end to end.
const DEMO_TOKEN = "demo-reset-token";

export function ForgotPassword() {
    const [email, setEmail] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [sent, setSent] = useState(false);

    function onSubmit(event: Event) {
        event.preventDefault();
        if (!isValidEmail(email)) {
            setError("Enter a valid email address.");
            return;
        }
        setError(null);
        setSent(true);
    }

    return (
        <div className="hero min-h-screen bg-base-200">
            <div className="hero-content w-full max-w-md">
                <div className="card w-full bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h1 className="card-title text-3xl">Reset your password</h1>

                        {sent ? (
                            <>
                                <div role="alert" className="alert alert-success">
                                    <span>
                                        If an account exists for {email}, we have sent it a
                                        link to choose a new password.
                                    </span>
                                </div>
                                <p className="label">
                                    No email is actually sent in this build.{" "}
                                    <a className="link" href={resetPasswordRoute(DEMO_TOKEN)}>
                                        Open the reset page
                                    </a>{" "}
                                    to continue.
                                </p>
                            </>
                        ) : (
                            <>
                                <p className="text-base-content/70">
                                    Enter the email you signed up with and we will send you a
                                    link to choose a new one.
                                </p>
                                {/* noValidate: native validation would block
                                    submit before onSubmit runs. */}
                                <form onSubmit={onSubmit} noValidate>
                                    <fieldset className="fieldset">
                                        <label className="input w-full">
                                            <span className="label">Email</span>
                                            <input
                                                type="email"
                                                autocomplete="username"
                                                value={email}
                                                onInput={event =>
                                                    setEmail(event.currentTarget.value)
                                                }
                                            />
                                        </label>
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
                                        Send reset link
                                    </button>
                                </form>
                            </>
                        )}

                        <p className="label mt-2">
                            <a className="link" href={Routes.Login}>Back to login</a>
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
}
