import {useLocation} from "preact-iso";
import {Routes} from "../routes";
import {useAuth} from "../auth";

export function AuthenticatedHeader() {
    const location = useLocation();
    const auth = useAuth();

    function onLogout() {
        auth.logout();
        location.route(Routes.Home);
    }

    return (
        <header className="navbar bg-base-200">
            <div className="navbar-start">
                <span className="text-xl font-semibold">Creator's Reward</span>
            </div>
            <div className="navbar-end gap-2">
                {auth.email !== null && (
                    <span className="hidden text-sm text-base-content/70 sm:inline">
                        {auth.email}
                    </span>
                )}
                <button type="button" className="btn" onClick={onLogout}>
                    Logout
                </button>
            </div>
        </header>
    );
}
