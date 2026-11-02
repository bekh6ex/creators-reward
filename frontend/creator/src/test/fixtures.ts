import {vi} from 'vitest';
import type {Website} from '../types';

// One website in each verification state. The API only returns verified sites
// for now, but the portal still renders all three.
export const demoWebsites: Website[] = [
	{
		id: '1',
		domain: 'daily-ledger.example',
		title: 'The Daily Ledger',
		addedAt: '2024-01-12',
		status: 'verified',
		token: 'crv-0d1a2b3c',
		lastCheckedAt: '2024-04-02T09:12:00Z',
	},
	{
		id: '2',
		domain: 'kompas-weekly.example',
		title: 'Kompas Weekly',
		addedAt: '2024-03-28',
		status: 'pending',
		token: 'crv-4e5f6a7b',
		lastCheckedAt: null,
	},
	{
		id: '3',
		domain: 'riverside-notes.example',
		title: 'Riverside Notes',
		addedAt: '2024-04-01',
		status: 'failed',
		token: 'crv-8c9d0e1f',
		lastCheckedAt: '2024-04-01T16:40:00Z',
	},
];

// Answers the next requests in order, then 401 for anything after them.
export function respondWith(...responses: Array<{status: number; body?: unknown}>) {
	const fetchMock = vi.fn(async () => new Response(null, {status: 401}));
	for (const {status, body} of responses) {
		fetchMock.mockImplementationOnce(
			async () =>
				new Response(body === undefined ? null : JSON.stringify(body), {status}),
		);
	}
	vi.stubGlobal('fetch', fetchMock);
	return fetchMock;
}

// Answers by request path; 401 for any other path, which reads as "not logged in".
export function routeFetch(routes: Record<string, {status: number; body?: unknown}>) {
	const fetchMock = vi.fn(async (path: string) => {
		const route = routes[path];
		if (route === undefined) return new Response(null, {status: 401});
		const body = route.body === undefined ? null : JSON.stringify(route.body);
		return new Response(body, {status: route.status});
	});
	vi.stubGlobal('fetch', fetchMock);
	return fetchMock;
}
