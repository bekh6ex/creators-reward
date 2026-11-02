import {Routes} from "../routes";

export function Menu() {
    return <nav aria-label="Main">
        <ul className="menu w-full">
            <li><a href={Routes.Websites}>Websites</a></li>
            <li><a href={Routes.Payouts}>Payouts</a></li>
            <li><a href={Routes.Profile}>Profile</a></li>
        </ul>
    </nav>;
}
