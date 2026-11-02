import type {ComponentChildren} from "preact";
import {useEffect} from "preact/hooks";
import {useLocation} from "preact-iso";
import {Menu} from "../components/Menu";
import {AuthenticatedHeader} from "../components/AuthenticatedHeader";
import {useAuth} from "../auth";
import {Routes} from "../routes";

export function AuthenticatedLayout(props: {children: ComponentChildren}) {
    const auth = useAuth();
    const location = useLocation();

    useEffect(() => {
        if (!auth.isLoading && !auth.isLoggedIn) {
            location.route(Routes.Home);
        }
    }, [auth.isLoading, auth.isLoggedIn, location]);

    if (auth.isLoading) {
        return (
            <div className="flex min-h-screen items-center justify-center bg-base-100">
                <span className="loading loading-spinner loading-lg" aria-label="Loading"></span>
            </div>
        );
    }

    if (!auth.isLoggedIn) {
        return null;
    }

    return <div className="flex min-h-screen flex-col bg-base-100">
        <AuthenticatedHeader/>
        <div className="flex grow flex-col md:flex-row">
            <aside className="w-full bg-base-200 md:w-64 md:shrink-0">
                <Menu/>
            </aside>
            <main className="grow p-4">
                {props.children}
            </main>
        </div>
    </div>;
}
