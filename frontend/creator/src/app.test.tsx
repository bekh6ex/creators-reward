import {describe, expect, it} from 'vitest';
import {fireEvent, render} from '@testing-library/preact';
import {App} from './app';
import {routeFetch} from './test/fixtures';

function logIn(container: ReturnType<typeof render>) {
	fireEvent.input(container.getByLabelText('Email'), {
		target: {value: 'owner@example.com'},
	});
	fireEvent.input(container.getByLabelText('Password'), {
		target: {value: 'secret123'},
	});
	fireEvent.click(container.getByRole('button', {name: 'Log in'}));
}

describe('App routing', () => {
	it('lands a logged-out visitor on the pitch', () => {
		window.history.replaceState(null, '', '/');
		const {getByRole} = render(<App/>);

		expect(
			getByRole('heading', {name: 'Get paid for what you publish'}),
		).toBeInTheDocument();
	});

	it('shows the website list after logging in', async () => {
		routeFetch({
			'/api/auth/login': {status: 204},
			'/api/websites': {status: 200, body: []},
		});
		window.history.replaceState(null, '', '/');
		const screen = render(<App/>);

		logIn(screen);

		expect(
			await screen.findByRole('heading', {name: 'Your websites'}),
		).toBeInTheDocument();
	});

	it('keeps a creator logged in across a reload', async () => {
		routeFetch({
			'/api/auth/session': {status: 200, body: {email: 'owner@example.com'}},
			'/api/websites': {status: 200, body: []},
		});
		window.history.replaceState(null, '', '/websites');
		const screen = render(<App/>);

		expect(
			await screen.findByRole('heading', {name: 'Your websites'}),
		).toBeInTheDocument();
	});

	// Regression: preact-iso keeps the first matching Route, so declaring
	// "/websites/:id" before "/websites/new" would render the detail page for a
	// website whose id is literally "new".
	it('treats /websites/new as the add page, not a website id', async () => {
		routeFetch({
			'/api/auth/login': {status: 204},
			'/api/websites': {status: 200, body: []},
		});
		window.history.replaceState(null, '', '/');
		const screen = render(<App/>);
		logIn(screen);

		fireEvent.click(await screen.findByRole('link', {name: 'Add website'}));

		expect(
			screen.getByRole('heading', {name: 'Add a website'}),
		).toBeInTheDocument();
	});

	it('falls back to the 404 page for an unknown path', () => {
		window.history.replaceState(null, '', '/nope');
		const {getByRole} = render(<App/>);

		expect(getByRole('heading', {name: '404'})).toBeInTheDocument();
	});

	it('keeps a logged-out visitor out of the portal', () => {
		window.history.replaceState(null, '', '/payouts');
		const {queryByRole} = render(<App/>);

		expect(queryByRole('heading', {name: 'Payouts'})).not.toBeInTheDocument();
	});
});
