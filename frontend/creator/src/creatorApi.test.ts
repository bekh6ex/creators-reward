import {describe, expect, it} from 'vitest';
import {
	CreatorApiError,
	addWebsite,
	fetchSession,
	listWebsites,
	login,
	logout,
	register,
} from './creatorApi';
import {demoWebsites, respondWith} from './test/fixtures';

describe('creatorApi', () => {
	it('registers with email and password as JSON', async () => {
		const fetchMock = respondWith({status: 201});

		await register('owner@example.org', 'secret123');

		expect(fetchMock).toHaveBeenCalledWith('/api/auth/register', {
			method: 'POST',
			headers: {'Content-Type': 'application/json'},
			body: JSON.stringify({email: 'owner@example.org', password: 'secret123'}),
			credentials: 'same-origin',
		});
	});

	it('carries the status of a refused request', async () => {
		respondWith({status: 409});

		const failure = await register('owner@example.org', 'secret123').catch(
			(error: unknown) => error,
		);

		expect(failure).toBeInstanceOf(CreatorApiError);
		expect((failure as CreatorApiError).status).toBe(409);
	});

	it('logs in and out', async () => {
		const fetchMock = respondWith({status: 204}, {status: 204});

		await login('owner@example.org', 'secret123');
		await logout();

		expect(fetchMock).toHaveBeenNthCalledWith(
			2,
			'/api/auth/logout',
			{method: 'POST', credentials: 'same-origin'},
		);
	});

	it('rejects wrong credentials and a failed logout', async () => {
		respondWith({status: 401}, {status: 500});

		await expect(login('owner@example.org', 'nope')).rejects.toThrow('Login failed: 401');
		await expect(logout()).rejects.toThrow('Logout failed: 500');
	});

	it('reads the session email, or null without a session', async () => {
		respondWith({status: 200, body: {email: 'owner@example.org'}}, {status: 401});

		expect(await fetchSession()).toBe('owner@example.org');
		expect(await fetchSession()).toBeNull();
	});

	it('treats any other session failure as an error', async () => {
		respondWith({status: 502});

		await expect(fetchSession()).rejects.toThrow('Session failed: 502');
	});

	it('lists and adds websites', async () => {
		const [website] = demoWebsites;
		const fetchMock = respondWith(
			{status: 200, body: demoWebsites},
			{status: 201, body: website},
		);

		expect(await listWebsites()).toEqual(demoWebsites);
		expect(await addWebsite('daily-ledger.example', '')).toEqual(website);
		expect(fetchMock).toHaveBeenNthCalledWith(
			2,
			'/api/websites',
			expect.objectContaining({
				body: JSON.stringify({domain: 'daily-ledger.example', title: ''}),
			}),
		);
	});

	it('rejects website calls the API refuses', async () => {
		respondWith({status: 401}, {status: 409});

		await expect(listWebsites()).rejects.toThrow('Listing websites failed: 401');
		await expect(addWebsite('daily-ledger.example', '')).rejects.toThrow(
			'Adding the website failed: 409',
		);
	});
});
