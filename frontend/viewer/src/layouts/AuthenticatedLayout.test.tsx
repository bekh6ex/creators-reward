import {afterEach, describe, expect, it, vi} from 'vitest';
import {AuthenticatedLayout} from './AuthenticatedLayout';
import {renderLoggedIn, renderLoggedOut, renderWithLocation} from '../test/render';
import {AuthProvider} from '../auth';
import {fetchSession} from '../authApi';
import {waitFor} from '@testing-library/preact';
import {Router, Route} from 'preact-iso';
import {Routes} from '../routes';

vi.mock('../authApi', () => ({
	fetchSession: vi.fn(),
	logout: vi.fn(),
}));

vi.mock('../viewerKeys', async () => {
	const storedKey = await crypto.subtle.generateKey({name: 'AES-GCM', length: 256}, false, [
		'encrypt',
	]);
	return {
		loadViewerPrivateKey: vi.fn().mockResolvedValue(storedKey),
		clearViewerPrivateKey: vi.fn().mockResolvedValue(undefined),
	};
});

describe('AuthenticatedLayout', () => {
	afterEach(() => {
		vi.mocked(fetchSession).mockReset();
		history.pushState(null, '', '/');
	});

	it('renders the page content inside the main landmark', () => {
		const {getByRole} = renderLoggedIn(
			<AuthenticatedLayout>
				<p>page content</p>
			</AuthenticatedLayout>,
		);

		expect(getByRole('main')).toHaveTextContent('page content');
	});

	it('renders the header and navigation around the content', () => {
		const {getByRole, getByText} = renderLoggedIn(
			<AuthenticatedLayout>
				<p>page content</p>
			</AuthenticatedLayout>,
		);

		expect(getByText("Creator's Reward")).toBeInTheDocument();
		expect(getByRole('navigation')).toBeInTheDocument();
	});

	it('renders nothing for a logged-out viewer and redirects to Home', async () => {
		history.pushState(null, '', '/dashboard');
		vi.mocked(fetchSession).mockResolvedValue(null);

		const {queryByRole, queryByText, queryByTestId, getByTestId} = renderLoggedOut(
			<Router>
				<Route path={Routes.Home} component={() => <div data-testid="home-page">Home Page</div>} />
				<Route default component={() => (
					<AuthenticatedLayout>
						<p>page content</p>
					</AuthenticatedLayout>
				)} />
			</Router>
		);

		await waitFor(() => {
			expect(queryByTestId('loading-spinner')).not.toBeInTheDocument();
		});

		expect(queryByText('page content')).not.toBeInTheDocument();
		expect(queryByRole('main')).not.toBeInTheDocument();

		await waitFor(() => {
			expect(getByTestId('home-page')).toBeInTheDocument();
		});
	});

	it('waits for session fetch before redirecting or rendering', async () => {
		history.pushState(null, '', '/dashboard');
		let resolveFetch: (val: any) => void;
		const fetchPromise = new Promise((resolve) => {
			resolveFetch = resolve;
		});
		vi.mocked(fetchSession).mockReturnValue(fetchPromise as any);

		const {getByTestId, queryByText, queryByRole, queryByTestId} = renderWithLocation(
			<AuthProvider>
				<Router>
					<Route path={Routes.Home} component={() => <div data-testid="home-page">Home Page</div>} />
					<Route default component={() => (
						<AuthenticatedLayout>
							<p>page content</p>
						</AuthenticatedLayout>
					)} />
				</Router>
			</AuthProvider>
		);

		expect(getByTestId('loading-spinner')).toBeInTheDocument();
		expect(queryByText('page content')).not.toBeInTheDocument();

		resolveFetch!({email: 'alice@example.com'});

		await waitFor(() => {
			expect(queryByText('page content')).toBeInTheDocument();
			expect(queryByRole('main')).toBeInTheDocument();
		});

		expect(queryByTestId('home-page')).not.toBeInTheDocument();
	});
});
