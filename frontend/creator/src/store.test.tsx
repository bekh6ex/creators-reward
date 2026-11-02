import {describe, expect, it} from 'vitest';
import {render, waitFor} from '@testing-library/preact';
import {AuthProvider} from './auth';
import {PortalProvider, usePortal} from './store';
import {demoWebsites, respondWith} from './test/fixtures';

function Domains() {
	const {websites} = usePortal();
	return <p>{websites.map(website => website.domain).join(', ') || 'none'}</p>;
}

describe('PortalProvider', () => {
	it('loads the websites of the logged-in creator', async () => {
		const fetchMock = respondWith({status: 200, body: demoWebsites.slice(0, 1)});

		const {getByText} = render(
			<AuthProvider initialEmail="owner@example.org">
				<PortalProvider><Domains/></PortalProvider>
			</AuthProvider>,
		);

		await waitFor(() => expect(getByText('daily-ledger.example')).toBeInTheDocument());
		expect(fetchMock).toHaveBeenCalledWith('/api/websites', {
			credentials: 'same-origin',
		});
	});

	it('loads nothing while logged out', async () => {
		const fetchMock = respondWith({status: 401});

		const {getByText} = render(
			<AuthProvider>
				<PortalProvider><Domains/></PortalProvider>
			</AuthProvider>,
		);

		await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
		expect(fetchMock).toHaveBeenCalledWith('/api/auth/session', {
			credentials: 'same-origin',
		});
		expect(getByText('none')).toBeInTheDocument();
	});

	it('shows an empty list when the websites cannot be loaded', async () => {
		const fetchMock = respondWith({status: 502});

		const {getByText} = render(
			<AuthProvider initialEmail="owner@example.org">
				<PortalProvider><Domains/></PortalProvider>
			</AuthProvider>,
		);

		await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
		expect(getByText('none')).toBeInTheDocument();
	});

	it('cannot be used outside the provider', () => {
		expect(() => render(<Domains/>)).toThrow(
			'usePortal must be used inside <PortalProvider>',
		);
	});
});
