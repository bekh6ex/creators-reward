import {LocationProvider, Route, Router} from 'preact-iso';
import {Routes} from "./routes";
import {AuthProvider} from "./auth";
import {PortalProvider} from "./store";
import {Home} from "./pages/Home";
import {Login} from "./pages/Login";
import {Register} from "./pages/Register";
import {ForgotPassword} from "./pages/ForgotPassword";
import {ResetPasswordRoute} from "./pages/ResetPassword";
import {Websites} from "./pages/Websites";
import {AddWebsite} from "./pages/AddWebsite";
import {WebsiteDetailRoute} from "./pages/WebsiteDetail";
import {Payouts} from "./pages/Payouts";
import {Profile} from "./pages/Profile";
import {NotFound} from "./pages/_404";

export function App() {
    return (
        <LocationProvider>
            <AuthProvider>
                <PortalProvider>
                    <Router>
                        <Route path={Routes.Home} component={Home}/>
                        <Route path={Routes.Login} component={Login}/>
                        <Route path={Routes.Register} component={Register}/>
                        <Route path={Routes.ForgotPassword} component={ForgotPassword}/>
                        <Route path={Routes.ResetPassword} component={ResetPasswordRoute}/>
                        <Route path={Routes.Websites} component={Websites}/>
                        {/* Before Routes.Website: preact-iso takes the first matching
                            child, so ":id" would otherwise swallow "/websites/new". */}
                        <Route path={Routes.AddWebsite} component={AddWebsite}/>
                        <Route path={Routes.Website} component={WebsiteDetailRoute}/>
                        <Route path={Routes.Payouts} component={Payouts}/>
                        <Route path={Routes.Profile} component={Profile}/>
                        <Route default component={NotFound}/>
                    </Router>
                </PortalProvider>
            </AuthProvider>
        </LocationProvider>
    );
}
