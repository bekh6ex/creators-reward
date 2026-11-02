import {Routes} from "../../routes";
import {LoginForm} from "../../components/LoginForm";

const STEPS: {title: string; body: string}[] = [
    {
        title: "Add your website",
        body: "Tell us the domain. We give you a meta tag to paste into your pages and fetch it to confirm the site is yours.",
    },
    {
        title: "Readers donate what they choose",
        body: "What readers read decides how much they donate to each site, up to a monthly limit each reader sets. Nothing on your site is ever locked, and reading never creates a debt.",
    },
    {
        title: "We transfer it to your bank",
        body: "Add an EU bank account and we send one monthly transfer covering every donation allocated to you.",
    },
];

export function Home() {
    return (
        <div className="min-h-screen bg-base-200">
            <div className="mx-auto flex max-w-5xl flex-col gap-8 p-4 py-10 lg:flex-row lg:items-start">
                <div className="flex flex-col gap-6 lg:flex-1">
                    <div>
                        <h1 className="text-4xl font-bold">Get paid for what you publish</h1>
                        <p className="mt-3 text-base-content/70">
                            Creator's Reward forwards donations from readers who chose to
                            support what they read. You can never see who donates — or
                            whether a given visitor donates at all — and we never store
                            what readers read in a form that points back to them.
                        </p>
                    </div>

                    <ul className="steps steps-vertical">
                        {STEPS.map(step => (
                            <li key={step.title} className="step">
                                <div className="text-left">
                                    <p className="font-semibold">{step.title}</p>
                                    <p className="text-sm text-base-content/70">{step.body}</p>
                                </div>
                            </li>
                        ))}
                    </ul>
                </div>

                <div className="card bg-base-100 shadow-sm lg:w-96">
                    <div className="card-body">
                        <h2 className="card-title">Log in</h2>
                        <LoginForm/>
                        <p className="label mt-2">
                            New here?{" "}
                            <a className="link" href={Routes.Register}>Create an account</a>
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
}
