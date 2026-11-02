import {Routes, websiteRoute} from "../../routes";
import {AuthenticatedLayout} from "../../layouts/AuthenticatedLayout";
import {VerificationBadge} from "../../components/VerificationBadge";
import {Link} from "../../components/Link";
import {usePortal} from "../../store";
import {formatDate, formatMoney, getWebsiteStats} from "../../api";

export function Websites() {
    const {websites} = usePortal();

    return (
        <AuthenticatedLayout>
            <div className="flex flex-col gap-4">
                <div className="flex flex-wrap items-center justify-between gap-2">
                    <h1 className="text-2xl font-semibold">Your websites</h1>
                    <a className="btn btn-primary" href={Routes.AddWebsite}>Add website</a>
                </div>

                {websites.length === 0 ? (
                    <div role="alert" className="alert">
                        <span>
                            You have not added a website yet. Add one to start earning.
                        </span>
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="table">
                            <thead>
                            <tr>
                                <th>Website</th>
                                <th>Domain</th>
                                <th>Status</th>
                                <th>Added</th>
                                <th>Earned</th>
                            </tr>
                            </thead>
                            <tbody>
                            {websites.map(website => (
                                <tr key={website.id}>
                                    <td>
                                        <Link href={websiteRoute(website.id)}>
                                            {website.title}
                                        </Link>
                                    </td>
                                    <td className="font-mono text-xs">{website.domain}</td>
                                    <td><VerificationBadge status={website.status}/></td>
                                    <td>{formatDate(website.addedAt)}</td>
                                    <td>{formatMoney(getWebsiteStats(website.id).earned)}</td>
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
