import {Routes} from "../routes";


export function Menu() {

    return <nav aria-label="Main">
        <ul className="menu w-full">
            <li><a href={Routes.Dashboard}>Dashboard</a></li>
            <li><a href={Routes.EnabledDonations}>Enabled donations</a></li>
        </ul>
    </nav>
}
