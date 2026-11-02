import {useRoute} from "preact-iso";
import {Routes} from "../../routes";
import {AuthenticatedLayout} from "../../layouts/AuthenticatedLayout";
import {CopyableCode} from "../../components/CopyableCode";
import {VerificationBadge} from "../../components/VerificationBadge";
import {Link} from "../../components/Link";
import {usePortal} from "../../store";
import {
    formatDateTime,
    formatMoney,
    formatNumber,
    getWebsite,
    getWebsiteStats,
    metaTag,
    monthLabel,
} from "../../api";

export function WebsiteDetail(props: {id: string}) {
    const {websites} = usePortal();
    const website = getWebsite(websites, props.id);

    if (website === undefined) {
        return (
            <AuthenticatedLayout>
                <div className="card bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h1 className="card-title">Website not found</h1>
                        <p>No website of yours has the id "{props.id}".</p>
                        <div className="card-actions mt-2">
                            <a className="link" href={Routes.Websites}>
                                Back to your websites
                            </a>
                        </div>
                    </div>
                </div>
            </AuthenticatedLayout>
        );
    }

    const stats = getWebsiteStats(website.id);

    return (
        <AuthenticatedLayout>
            <div className="flex flex-col gap-4">
                <div>
                    <a className="link text-sm" href={Routes.Websites}>← Your websites</a>
                    <div className="mt-1 flex flex-wrap items-center gap-3">
                        <h1 className="text-2xl font-semibold">{website.title}</h1>
                        <VerificationBadge status={website.status}/>
                    </div>
                    <p className="text-sm text-base-content/70">
                        <Link href={`https://${website.domain}`} external>
                            {website.domain}
                        </Link>
                    </p>
                </div>

                <section className="card bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h2 className="card-title">Ownership</h2>

                        {website.status === "verified" ? (
                            <div role="alert" className="alert alert-success">
                                <span>
                                    {website.domain} is connected to your account. We do not
                                    check who owns a website yet; once we do, we look for this
                                    tag in the <code>&lt;head&gt;</code> of its home page.
                                </span>
                            </div>
                        ) : (
                            <p className="text-base-content/70">
                                Add this tag inside the <code>&lt;head&gt;</code> of your home
                                page. We fetch{" "}
                                <span className="font-mono text-xs">
                                    https://{website.domain}/
                                </span>{" "}
                                and look for it.
                            </p>
                        )}

                        <CopyableCode label="Meta tag" value={metaTag(website.token)}/>

                        {website.lastCheckedAt !== null && (
                            <p className="text-sm text-base-content/70">
                                Last checked {formatDateTime(website.lastCheckedAt)}
                            </p>
                        )}
                    </div>
                </section>

                <section className="flex flex-col gap-4">
                    <h2 className="text-xl font-semibold">Statistics</h2>

                    <div className="stats stats-vertical w-full bg-base-200 sm:stats-horizontal sm:w-auto sm:self-start">
                        <div className="stat">
                            <div className="stat-title">Reads</div>
                            <div className="stat-value text-2xl">
                                {formatNumber(stats.views)}
                            </div>
                        </div>
                        <div className="stat">
                            <div className="stat-title">Readers</div>
                            <div className="stat-value text-2xl">
                                {formatNumber(stats.viewers)}
                            </div>
                        </div>
                        <div className="stat">
                            <div className="stat-title">Earned</div>
                            <div className="stat-value text-2xl">
                                {formatMoney(stats.earned)}
                            </div>
                        </div>
                    </div>

                    {stats.views === 0 ? (
                        <div role="alert" className="alert">
                            <span>
                                No reads recorded yet. Statistics appear once readers using
                                Creator's Reward visit a verified site.
                            </span>
                        </div>
                    ) : (
                        <>
                            <div className="overflow-x-auto">
                                <table className="table">
                                    <caption className="text-left text-sm text-base-content/70">
                                        By month
                                    </caption>
                                    <thead>
                                    <tr>
                                        <th>Month</th>
                                        <th>Reads</th>
                                        <th>Readers</th>
                                        <th>Earned</th>
                                    </tr>
                                    </thead>
                                    <tbody>
                                    {stats.months.map(month => (
                                        <tr key={month.month}>
                                            <td>{monthLabel(month.month)}</td>
                                            <td>{formatNumber(month.views)}</td>
                                            <td>{formatNumber(month.viewers)}</td>
                                            <td>{formatMoney(month.earned)}</td>
                                        </tr>
                                    ))}
                                    </tbody>
                                </table>
                            </div>

                            <div className="overflow-x-auto">
                                <table className="table">
                                    <caption className="text-left text-sm text-base-content/70">
                                        Most read pages
                                    </caption>
                                    <thead>
                                    <tr>
                                        <th>Page</th>
                                        <th>Reads</th>
                                        <th>Earned</th>
                                    </tr>
                                    </thead>
                                    <tbody>
                                    {stats.topPages.map(page => (
                                        <tr key={page.url}>
                                            <td>
                                                <Link href={page.url} external>
                                                    {page.title}
                                                </Link>
                                            </td>
                                            <td>{formatNumber(page.views)}</td>
                                            <td>{formatMoney(page.earned)}</td>
                                        </tr>
                                    ))}
                                    </tbody>
                                </table>
                            </div>

                            <p className="text-sm text-base-content/70">
                                Reader counts are totals, never lists. We cannot tell you who
                                read a page.
                            </p>
                        </>
                    )}
                </section>
            </div>
        </AuthenticatedLayout>
    );
}

export function WebsiteDetailRoute() {
    const {params} = useRoute();
    return <WebsiteDetail id={params["id"] ?? ""}/>;
}
