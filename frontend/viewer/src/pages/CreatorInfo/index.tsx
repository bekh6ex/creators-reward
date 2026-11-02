import {useRoute} from "preact-iso";
import {Routes} from "../../routes";
import {getCreator} from "../../api";

export function CreatorInfo(props: {id: string}) {
    const creator = getCreator(props.id);

    return (
        <div className="mx-auto max-w-3xl p-4">
            <div className="card bg-base-100 shadow-sm">
                <div className="card-body">
                    <h1 className="card-title text-3xl">
                        {creator?.title ?? "Creator not found"}
                    </h1>
                    <p>
                        {creator?.description ??
                            `No creator matches the id "${props.id}".`}
                    </p>
                    <div className="card-actions mt-2">
                        <a className="link" href={Routes.Dashboard}>Back to dashboard</a>
                    </div>
                </div>
            </div>
        </div>
    );
}

// preact-iso exposes route params through useRoute() rather than as props, so
// the page stays a plain presentational component and this adapter is what the
// Router mounts.
export function CreatorInfoRoute() {
    const {params} = useRoute();
    return <CreatorInfo id={params["id"] ?? ""}/>;
}
