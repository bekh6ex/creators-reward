import {describe, expect, it, vi} from 'vitest';
import {fireEvent, screen, waitFor} from '@testing-library/preact';
import {Login} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';
import {login, AuthApiError} from '../../authApi';

vi.mock('../../authApi', () => ({
	login: vi.fn(),
	fetchSession: vi.fn().mockResolvedValue(null),
	AuthApiError: class extends Error {
		status: number | undefined;
		constructor(message: string, status?: number) {
			super(message);
			this.name = 'AuthApiError';
			this.status = status;
		}
	},
}));

describe('Login', () => {
	it('shows the login heading', () => {
		const {getByRole} = renderLoggedOut(<Login/>);

		expect(getByRole('heading', {name: 'Log in'})).toBeInTheDocument();
	});

	it('renders email and password fields', () => {
		const {getByLabelText} = renderLoggedOut(<Login/>);

		expect(getByLabelText('Email')).toBeInTheDocument();
		expect(getByLabelText('Password')).toHaveAttribute('type', 'password');
	});

	it('offers a link to registration', () => {
		const {getByRole} = renderLoggedOut(<Login/>);

		expect(getByRole('link', {name: 'Create one'})).toHaveAttribute(
			'href',
			Routes.Register,
		);
	});

	it('navigates to dashboard on successful login', async () => {
		const pushState = vi.spyOn(window.history, 'pushState');
		vi.mocked(login).mockResolvedValueOnce(undefined);
		renderLoggedOut(<Login/>);

		fireEvent.input(screen.getByLabelText('Email'), {
			target: {value: 'test@example.com'},
		});
		fireEvent.input(screen.getByLabelText('Password'), {
			target: {value: 'password123'},
		});
		fireEvent.click(screen.getByRole('button', {name: /sign in/i}));

		await waitFor(() => {
			expect(pushState).toHaveBeenCalledWith(null, '', Routes.Dashboard);
		});
		pushState.mockRestore();
	});

	it('shows error on wrong password', async () => {
		const pushState = vi.spyOn(window.history, 'pushState');
		vi.mocked(login).mockRejectedValueOnce(
			new AuthApiError('Invalid credentials', 401),
		);
		renderLoggedOut(<Login/>);

		fireEvent.input(screen.getByLabelText('Email'), {
			target: {value: 'test@example.com'},
		});
		fireEvent.input(screen.getByLabelText('Password'), {
			target: {value: 'wrong'},
		});
		fireEvent.click(screen.getByRole('button', {name: /sign in/i}));

		await waitFor(() => {
			expect(
				screen.getByText(/invalid email or password/i),
			).toBeInTheDocument();
		});
		expect(pushState).not.toHaveBeenCalled();
		pushState.mockRestore();
	});

	it('disables button while logging in', async () => {
		vi.mocked(login).mockImplementation(() => new Promise(() => {}));
		renderLoggedOut(<Login/>);

		fireEvent.input(screen.getByLabelText('Email'), {
			target: {value: 'test@example.com'},
		});
		fireEvent.input(screen.getByLabelText('Password'), {
			target: {value: 'password123'},
		});
		fireEvent.click(screen.getByRole('button', {name: /sign in/i}));

		await waitFor(() => {
			expect(
				screen.getByRole('button', {name: /signing in/i}),
			).toBeDisabled();
		});
	});

	it('complains instead of navigating when the form is empty', () => {
		const pushState = vi.spyOn(window.history, 'pushState');
		renderLoggedOut(<Login/>);

		fireEvent.click(screen.getByRole('button', {name: /sign in/i}));

		expect(screen.getByRole('alert')).toHaveTextContent(
			/please fill in all fields/i,
		);
		expect(pushState).not.toHaveBeenCalled();
		pushState.mockRestore();
	});
});
