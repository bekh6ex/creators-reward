import {describe, expect, it, vi} from 'vitest';
import {fireEvent, waitFor} from '@testing-library/preact';
import {Register} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';
import {routeFetch} from '../../test/fixtures';

function fill(
	getByLabelText: (text: string) => HTMLElement,
	email: string,
	password: string,
	confirmation: string,
) {
	fireEvent.input(getByLabelText('Email'), {target: {value: email}});
	fireEvent.input(getByLabelText('Password'), {target: {value: password}});
	fireEvent.input(getByLabelText('Repeat password'), {
		target: {value: confirmation},
	});
}

describe('Register', () => {
	it('rejects an address that is not an email', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'not-an-email', 'secret123', 'secret123');
		fireEvent.click(getByRole('button', {name: 'Create account'}));

		expect(getByRole('alert')).toHaveTextContent('valid email address');
	});

	it('rejects a password shorter than eight characters', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'owner@example.com', 'short', 'short');
		fireEvent.click(getByRole('button', {name: 'Create account'}));

		expect(getByRole('alert')).toHaveTextContent('at least 8 characters');
	});

	it('rejects mismatched passwords', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'owner@example.com', 'secret123', 'secret124');
		fireEvent.click(getByRole('button', {name: 'Create account'}));

		expect(getByRole('alert')).toHaveTextContent('do not match');
	});

	it('reports an email that is already registered', async () => {
		routeFetch({'/api/auth/register': {status: 409}});
		const {getByLabelText, findByRole, getByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'taken@example.com', 'secret123', 'secret123');
		fireEvent.click(getByRole('button', {name: 'Create account'}));

		expect(await findByRole('alert')).toHaveTextContent('already registered');
	});

	it('reports a registration the backend could not complete', async () => {
		routeFetch({'/api/auth/register': {status: 503}});
		const {getByLabelText, findByRole, getByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'owner@example.com', 'secret123', 'secret123');
		fireEvent.click(getByRole('button', {name: 'Create account'}));

		expect(await findByRole('alert')).toHaveTextContent('Could not create the account');
	});

	it('registers and goes to the website list', async () => {
		const fetchMock = routeFetch({'/api/auth/register': {status: 201}});
		const pushState = vi.spyOn(window.history, 'pushState');
		const {getByLabelText, getByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'owner@example.com', 'secret123', 'secret123');
		fireEvent.click(getByRole('button', {name: 'Create account'}));

		await waitFor(() =>
			expect(pushState).toHaveBeenCalledWith(null, '', Routes.Websites),
		);
		expect(fetchMock).toHaveBeenCalledWith(
			'/api/auth/register',
			expect.objectContaining({
				body: JSON.stringify({email: 'owner@example.com', password: 'secret123'}),
			}),
		);
		pushState.mockRestore();
	});

	// The viewer SPA warns that a forgotten password is unrecoverable, because
	// there it derives the history encryption key. Nothing in this portal is
	// encrypted with it, so recovery is offered instead.
	it('does not claim the password is unrecoverable', () => {
		const {queryByText} = renderLoggedOut(<Register/>);

		expect(queryByText(/cannot reset/i)).not.toBeInTheDocument();
	});

	it('links back to login', () => {
		const {getByRole} = renderLoggedOut(<Register/>);

		expect(getByRole('link', {name: 'Log in'})).toHaveAttribute(
			'href',
			Routes.Login,
		);
	});
});
