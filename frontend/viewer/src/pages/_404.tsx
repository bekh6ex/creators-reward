import {Routes} from "../routes";

export function NotFound() {
	return (
		<div className="hero min-h-screen bg-base-200">
			<div className="hero-content text-center">
				<div>
					<h1 className="text-5xl font-bold">404</h1>
					<p className="py-6">We could not find that page.</p>
					<a className="btn" href={Routes.Home}>Go home</a>
				</div>
			</div>
		</div>
	);
}
