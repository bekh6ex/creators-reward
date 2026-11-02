import {Routes} from "../../routes";
import {LoginForm} from "../../components/LoginForm";

const highlights = [
    {
        title: "Donations, not subscriptions",
        description:
            "Nothing is ever locked. Every article stays readable whether you give 50 eur a month or nothing at all.",
    },
    {
        title: "Untrackable by design",
        description:
            "Your history is encrypted with a key derived from your password. We see the page you read only while processing it, and never store it in a form that points back to you.",
    },
    {
        title: "You set the budget",
        description:
            "What you read decides your donation, up to a monthly limit you choose. Once you reach it, every share is scaled down proportionally, so you never give more than you chose.",
    },
];

export function Home() {
    return (
        <div className="hero min-h-screen bg-base-200">
            <div className="hero-content w-full max-w-6xl flex-col items-stretch gap-10 py-12 lg:flex-row lg:items-start">
                <div className="w-full lg:w-3/5">
                    <h1 className="text-4xl font-bold">Creator's Reward</h1>
                    <p className="mt-4 max-w-prose text-base-content/70">
                        Donate to the people who wrote what you just read — without
                        building a record of your reading.
                    </p>
                    <div className="mt-8 flex flex-col gap-4">
                        {highlights.map(highlight => (
                            <div key={highlight.title} className="card bg-base-100 shadow-sm">
                                <div className="card-body">
                                    <h2 className="card-title text-lg">{highlight.title}</h2>
                                    <p>{highlight.description}</p>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                <div className="w-full lg:w-2/5 lg:max-w-sm">
                    <div className="card bg-base-100 shadow-sm">
                        <div className="card-body">
                            <h2 className="card-title text-2xl">Log in</h2>
                            <LoginForm/>
                            <p className="label mt-2">
                                No account yet?{" "}
                                <a className="link" href={Routes.Register}>Create one</a>
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
