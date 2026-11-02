import {useState} from "preact/hooks";
import {creatorInfoRoute} from "../../routes";
import {Link} from "../../components/Link";
import {AuthenticatedLayout} from "../../layouts/AuthenticatedLayout";
import {
    formatDateTime,
    formatMoney,
    initialViews,
    monthKey,
    monthLabel,
    totalCost,
} from "../../api";
import type {Creator, ViewItem} from "../../types";

const ALL = "";

export function Dashboard() {
    const [views, setViews] = useState<ViewItem[]>(initialViews);
    const [creatorId, setCreatorId] = useState(ALL);
    const [month, setMonth] = useState(ALL);

    const creators: Creator[] = [
        ...new Map(views.map(view => [view.creator.id, view.creator])).values(),
    ];
    const months = [...new Set(views.map(view => monthKey(view.at)))].toSorted(
        (left, right) => right.localeCompare(left),
    );

    // Both filters run in the browser: the creator of a view lives inside the
    // encrypted payload, so the server has no way to filter on it and returns
    // everything the viewer is entitled to see.
    const filtered = views.filter(
        view =>
            (creatorId === ALL || view.creator.id === creatorId) &&
            (month === ALL || monthKey(view.at) === month),
    );

    function deleteView(id: string) {
        setViews(current => current.filter(view => view.id !== id));
    }

    return (
        <AuthenticatedLayout>
            <div className="flex flex-col gap-4">
                <div className="stats stats-vertical w-full bg-base-200 sm:stats-horizontal sm:w-auto sm:self-start">
                    <div className="stat">
                        <div className="stat-title">Views</div>
                        <div className="stat-value text-2xl">{filtered.length}</div>
                    </div>
                    <div className="stat">
                        <div className="stat-title">Total</div>
                        <div className="stat-value text-2xl">{totalCost(filtered)}</div>
                    </div>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                    <span>Filter:</span>
                    <select
                        className="select w-auto"
                        aria-label="Filter by creator"
                        value={creatorId}
                        onChange={event => setCreatorId(event.currentTarget.value)}
                    >
                        <option value={ALL}>By creator</option>
                        {creators.map(creator => (
                            <option key={creator.id} value={creator.id}>{creator.title}</option>
                        ))}
                    </select>
                    <select
                        className="select w-auto"
                        aria-label="Filter by month"
                        value={month}
                        onChange={event => setMonth(event.currentTarget.value)}
                    >
                        <option value={ALL}>By month</option>
                        {months.map(key => (
                            <option key={key} value={key}>{monthLabel(key)}</option>
                        ))}
                    </select>
                </div>

                {filtered.length === 0 ? (
                    <div role="alert" className="alert">
                        <span>No views match this filter.</span>
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="table">
                            <thead>
                            <tr>
                                <th>Time</th>
                                <th>Creator</th>
                                <th>Article</th>
                                <th>Cost</th>
                                <th>Actions</th>
                            </tr>
                            </thead>
                            <tbody>
                            {filtered.map(item => (
                                <tr key={item.id}>
                                    <td>{formatDateTime(item.at)}</td>
                                    <td>
                                        <Link href={creatorInfoRoute(item.creator.id)}>
                                            {item.creator.title}
                                        </Link>
                                    </td>
                                    <td>
                                        <Link href={item.article.url} external>
                                            {item.article.title}
                                        </Link>
                                    </td>
                                    <td>{formatMoney(item.cost)}</td>
                                    <td>
                                        <button
                                            type="button"
                                            className="btn btn-sm"
                                            onClick={() => deleteView(item.id)}
                                        >
                                            Delete
                                        </button>
                                    </td>
                                </tr>
                            ))}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>
        </AuthenticatedLayout>
    );
}
