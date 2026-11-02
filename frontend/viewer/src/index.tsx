import {render} from 'preact';
import {LocationProvider, Route, Router} from 'preact-iso';
import './style.css';
import {Routes} from "./routes";
import {AuthProvider} from "./auth";
import {Home} from "./pages/Home";
import {Login} from "./pages/Login";
import {Register} from "./pages/Register";
import {Dashboard} from "./pages/Dashboard";
import {EnabledDonations} from "./pages/EnabledDonations";
import {CreatorInfoRoute} from "./pages/CreatorInfo";
import {NotFound} from "./pages/_404";

function App() {
    return (
        <LocationProvider>
            <AuthProvider>
                <Router>
                    <Route path={Routes.Home} component={Home}/>
                    <Route path={Routes.Login} component={Login}/>
                    <Route path={Routes.Register} component={Register}/>
                    <Route path={Routes.Dashboard} component={Dashboard}/>
                    <Route path={Routes.EnabledDonations} component={EnabledDonations}/>
                    <Route path={Routes.CreatorInfo} component={CreatorInfoRoute}/>
                    <Route default component={NotFound}/>
                </Router>
            </AuthProvider>
        </LocationProvider>
    );
}

render(<App/>, document.getElementById('app')!);
