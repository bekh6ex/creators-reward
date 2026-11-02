import {useId, useState} from "preact/hooks";
import {AuthenticatedLayout} from "../../layouts/AuthenticatedLayout";
import {usePortal} from "../../store";
import {isValidEmail} from "../../api";

export function Profile() {
    const {profile, saveProfile} = usePortal();
    const titleId = useId();
    const descriptionId = useId();
    const homepageId = useId();
    const emailId = useId();

    const [title, setTitle] = useState(profile.title);
    const [description, setDescription] = useState(profile.description);
    const [homepage, setHomepage] = useState(profile.homepage);
    const [contactEmail, setContactEmail] = useState(profile.contactEmail);
    const [error, setError] = useState<string | null>(null);
    const [saved, setSaved] = useState(false);

    function onSubmit(event: Event) {
        event.preventDefault();
        if (title.trim() === "") {
            setError("Readers need a name to recognise you by.");
            return;
        }
        if (homepage.trim() !== "" && !/^https?:\/\/\S+$/.test(homepage.trim())) {
            setError("The homepage must be a full address starting with https://");
            return;
        }
        if (!isValidEmail(contactEmail)) {
            setError("Enter a valid contact email address.");
            return;
        }
        setError(null);
        saveProfile({
            id: profile.id,
            title: title.trim(),
            description: description.trim(),
            homepage: homepage.trim(),
            contactEmail: contactEmail.trim(),
        });
        setSaved(true);
    }

    return (
        <AuthenticatedLayout>
            <div className="flex max-w-2xl flex-col gap-4">
                <h1 className="text-2xl font-semibold">About you</h1>

                <div className="card bg-base-100 shadow-sm">
                    <div className="card-body">
                        <p className="text-base-content/70">
                            Your name and description are public: readers see them on your
                            page before deciding to support you. Your contact address is not.
                        </p>

                        {saved && (
                            <div role="alert" className="alert alert-success">
                                <span>Profile saved.</span>
                            </div>
                        )}

                        {/* noValidate: native validation would block submit
                            before onSubmit runs. */}
                        <form onSubmit={onSubmit} noValidate>
                            <fieldset className="fieldset">
                                <label className="label" htmlFor={titleId}>Name</label>
                                <input
                                    id={titleId}
                                    type="text"
                                    className="input w-full"
                                    value={title}
                                    onInput={event => setTitle(event.currentTarget.value)}
                                />

                                <label className="label" htmlFor={descriptionId}>
                                    Description
                                </label>
                                <textarea
                                    id={descriptionId}
                                    className="textarea h-24 w-full"
                                    value={description}
                                    onInput={event =>
                                        setDescription(event.currentTarget.value)
                                    }
                                />

                                <label className="label" htmlFor={homepageId}>Homepage</label>
                                <input
                                    id={homepageId}
                                    type="url"
                                    className="input w-full"
                                    placeholder="https://example.org"
                                    value={homepage}
                                    onInput={event => setHomepage(event.currentTarget.value)}
                                />

                                <label className="label" htmlFor={emailId}>
                                    Contact email
                                </label>
                                <input
                                    id={emailId}
                                    type="email"
                                    className="input w-full"
                                    autocomplete="email"
                                    value={contactEmail}
                                    onInput={event =>
                                        setContactEmail(event.currentTarget.value)
                                    }
                                />
                                <p className="label">
                                    Only used if we need to reach you about payouts.
                                </p>
                            </fieldset>

                            {error !== null && (
                                <div role="alert" className="alert alert-error mt-4">
                                    <span>{error}</span>
                                </div>
                            )}

                            <button type="submit" className="btn btn-primary mt-4">
                                Save profile
                            </button>
                        </form>
                    </div>
                </div>
            </div>
        </AuthenticatedLayout>
    );
}
