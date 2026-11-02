import {Routes} from "../../routes";
import {LoginForm} from "../../components/LoginForm";

export function Login() {
    return (
        <div className="hero min-h-screen bg-base-200">
            <div className="hero-content w-full max-w-sm">
                <div className="card w-full bg-base-100 shadow-sm">
                    <div className="card-body">
                        <h1 className="card-title text-3xl">Log in</h1>
                        <LoginForm/>
                        <p className="label mt-2">
                            No account yet?{" "}
                            <a className="link" href={Routes.Register}>Create one</a>
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
}
