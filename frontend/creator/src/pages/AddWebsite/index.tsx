import {useState} from "preact/hooks";
import {Routes, websiteRoute} from "../../routes";
import {AuthenticatedLayout} from "../../layouts/AuthenticatedLayout";
import {CopyableCode} from "../../components/CopyableCode";
import {usePortal} from "../../store";
import {metaTag, normaliseDomain, validateDomain} from "../../api";
import {CreatorApiError} from "../../creatorApi";
import type {Website} from "../../types";

export function AddWebsite() {
    const {websites, addWebsite} = usePortal();
    const [domain, setDomain] = useState("");
    const [title, setTitle] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [created, setCreated] = useState<Website | null>(null);
    const [busy, setBusy] = useState(false);

    async function onSubmit(event: Event) {
        event.preventDefault();
        const cleaned = normaliseDomain(domain);
        const problem = validateDomain(cleaned);
        if (problem !== null) {
            setError(problem);
            return;
        }
        if (websites.some(website => website.domain === cleaned)) {
            setError("You have already added that website.");
            return;
        }
        setError(null);
        setBusy(true);
        try {
            setCreated(await addWebsite(cleaned, title.trim()));
        } catch (failure) {
            setError(
                failure instanceof CreatorApiError && failure.status === 409
                    ? "That website is already registered to an account."
                    : "Could not add the website. Try again in a moment.",
            );
        }
        setBusy(false);
    }

    return (
        <AuthenticatedLayout>
            <div className="mx-auto flex max-w-2xl flex-col gap-4">
                <h1 className="text-2xl font-semibold">Add a website</h1>

                {created === null ? (
                    <div className="card bg-base-100 shadow-sm">
                        <div className="card-body">
                            <p className="text-base-content/70">
                                Add the site you publish on. It is connected to your account
                                straight away.
                            </p>
                            <form onSubmit={onSubmit} noValidate>
                                <fieldset className="fieldset">
                                    <label className="input w-full">
                                        <span className="label">Domain</span>
                                        <input
                                            type="text"
                                            placeholder="example.org"
                                            value={domain}
                                            onInput={event =>
                                                setDomain(event.currentTarget.value)
                                            }
                                        />
                                    </label>
                                    <label className="input w-full">
                                        <span className="label">Name</span>
                                        <input
                                            type="text"
                                            placeholder="Optional — defaults to the domain"
                                            value={title}
                                            onInput={event =>
                                                setTitle(event.currentTarget.value)
                                            }
                                        />
                                    </label>
                                    <p className="label">
                                        Paste the address of your home page. We strip the
                                        https:// and any path for you.
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
                                    Add website
                                </button>
                            </form>
                        </div>
                    </div>
                ) : (
                    <div className="card bg-base-100 shadow-sm">
                        <div className="card-body">
                            <h2 className="card-title">{created.domain} is connected</h2>
                            <p className="text-base-content/70">
                                We do not check who owns a website yet. Once we do, we will
                                fetch{" "}
                                <span className="font-mono text-xs">
                                    https://{created.domain}/
                                </span>{" "}
                                and look for this tag inside its <code>&lt;head&gt;</code>. You
                                can add it now.
                            </p>

                            <CopyableCode
                                label="Meta tag"
                                value={metaTag(created.token)}
                            />

                            <div className="card-actions mt-4 justify-end">
                                <a className="btn" href={Routes.Websites}>Your websites</a>
                                <a
                                    className="btn btn-primary"
                                    href={websiteRoute(created.id)}
                                >
                                    Open {created.title}
                                </a>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        </AuthenticatedLayout>
    );
}
