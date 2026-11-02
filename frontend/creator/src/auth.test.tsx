import {describe, expect, it} from 'vitest';
import {render, waitFor} from '@testing-library/preact';
import {AuthProvider, useAuth} from './auth';
import {respondWith} from './test/fixtures';

function Probe() {
	const auth = useAuth();
	if (auth.isLoading) return <p>loading</p>;
	return <p>{auth.email ?? 'logged out'}</p>;
}

describe('AuthProvider', () => {
	it('restores the session the cookie still carries', async () => {
		respondWith({status: 200, body: {email: 'owner@example.org'}});

		const {getByText} = render(<AuthProvider><Probe/></AuthProvider>);

		expect(getByText('loading')).toBeInTheDocument();
		await waitFor(() => expect(getByText('owner@example.org')).toBeInTheDocument());
	});

	it('stays logged out without a session', async () => {
		const {getByText} = render(<AuthProvider><Probe/></AuthProvider>);

		await waitFor(() => expect(getByText('logged out')).toBeInTheDocument());
	});

	it('stays logged out when the backend is unreachable', async () => {
		respondWith({status: 502});

		const {getByText} = render(<AuthProvider><Probe/></AuthProvider>);

		await waitFor(() => expect(getByText('logged out')).toBeInTheDocument());
	});

	it('cannot be used outside the provider', () => {
		expect(() => render(<Probe/>)).toThrow('useAuth must be used inside <AuthProvider>');
	});
});
