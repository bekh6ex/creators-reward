import {describe, expect, it, vi} from 'vitest';
import {fireEvent, waitFor} from '@testing-library/preact';
import {Login} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';
import {routeFetch} from '../../test/fixtures';

function logIn(
	getByLabelText: (text: string) => HTMLElement,
	getByRole: (role: string, options: {name: string}) => HTMLElement,
) {
	fireEvent.input(getByLabelText('Email'), {
		target: {value: 'owner@example.com'},
	});
	fireEvent.input(getByLabelText('Password'), {target: {value: 'secret123'}});
	fireEvent.click(getByRole('button', {name: 'Log in'}));
}

describe('Login', () => {
	it('asks for credentials', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<Login/>);

		expect(getByRole('heading', {name: 'Log in'})).toBeInTheDocument();
		expect(getByLabelText('Email')).toBeInTheDocument();
		expect(getByLabelText('Password')).toBeInTheDocument();
	});

	it('refuses an empty form', () => {
		const {getByRole} = renderLoggedOut(<Login/>);

		fireEvent.click(getByRole('button', {name: 'Log in'}));

		expect(getByRole('alert')).toHaveTextContent('Enter your email and password.');
	});

	it('goes to the website list once logged in', async () => {
		routeFetch({'/api/auth/login': {status: 204}});
		const pushState = vi.spyOn(window.history, 'pushState');
		const {getByLabelText, getByRole} = renderLoggedOut(<Login/>);

		logIn(getByLabelText, getByRole);

		await waitFor(() =>
			expect(pushState).toHaveBeenCalledWith(null, '', Routes.Websites),
		);
		pushState.mockRestore();
	});

	it('refuses a wrong password', async () => {
		const {getByLabelText, getByRole, findByRole} = renderLoggedOut(<Login/>);

		logIn(getByLabelText, getByRole);

		expect(await findByRole('alert')).toHaveTextContent('Wrong email or password.');
	});

	it('says so when the backend cannot log anyone in', async () => {
		routeFetch({'/api/auth/login': {status: 503}});
		const {getByLabelText, getByRole, findByRole} = renderLoggedOut(<Login/>);

		logIn(getByLabelText, getByRole);

		expect(await findByRole('alert')).toHaveTextContent('Could not log in.');
	});

	it('links to registration', () => {
		const {getByRole} = renderLoggedOut(<Login/>);

		expect(getByRole('link', {name: 'Create an account'})).toHaveAttribute(
			'href',
			Routes.Register,
		);
	});
});
