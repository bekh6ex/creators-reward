import {describe, expect, it, vi} from 'vitest';
import {fireEvent, waitFor} from '@testing-library/preact';
import {Register} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';
import {register, AuthApiError} from '../../authApi';

vi.mock('../../authApi', () => ({
	register: vi.fn(),
	fetchSession: vi.fn(() => Promise.resolve(null)),
	logout: vi.fn(() => Promise.resolve()),
	AuthApiError: class extends Error {
		status: number | undefined;
		constructor(message: string, status?: number) {
			super(message);
			this.name = 'AuthApiError';
			this.status = status;
		}
	},
}));

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
	it('warns that a forgotten password cannot be recovered', () => {
		const {getByText} = renderLoggedOut(<Register/>);

		expect(
			getByText(/we cannot reset it/i),
		).toBeInTheDocument();
	});

	it('rejects a password shorter than eight characters', () => {
		const {getByLabelText, getAllByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'viewer@example.com', 'short', 'short');
		fireEvent.click(getAllByRole('button', {name: 'Create account'})[0]!);

		expect(
			getAllByRole('alert').some(alert =>
				alert.textContent?.includes('at least 8 characters'),
			),
		).toBe(true);
	});

	it('rejects mismatched passwords', () => {
		const {getByLabelText, getAllByRole} = renderLoggedOut(<Register/>);

		fill(getByLabelText, 'viewer@example.com', 'secret123', 'secret124');
		fireEvent.click(getAllByRole('button', {name: 'Create account'})[0]!);

		expect(
			getAllByRole('alert').some(alert =>
				alert.textContent?.includes('do not match'),
			),
		).toBe(true);
	});

	it('registers and navigates to dashboard', async () => {
		const pushState = vi.spyOn(window.history, 'pushState');
		vi.mocked(register).mockResolvedValueOnce(undefined);

		const {getByLabelText, getAllByRole} = renderLoggedOut(<Register/>);
		fill(getByLabelText, 'new@example.com', 'password123', 'password123');
		fireEvent.click(getAllByRole('button', {name: 'Create account'})[0]!);

		await waitFor(() => {
			expect(pushState).toHaveBeenCalledWith(null, '', Routes.Dashboard);
		});
		pushState.mockRestore();
	});

	it('shows already-registered error on 409', async () => {
		vi.mocked(register).mockRejectedValueOnce(
			new AuthApiError('Email already registered', 409),
		);

		const {getByLabelText, getAllByRole, getByText} = renderLoggedOut(<Register/>);
		fill(getByLabelText, 'existing@example.com', 'password123', 'password123');
		fireEvent.click(getAllByRole('button', {name: 'Create account'})[0]!);

		await waitFor(() => {
			expect(getByText(/already registered/i)).toBeInTheDocument();
		});
	});

	it('shows validation error on 422', async () => {
		vi.mocked(register).mockRejectedValueOnce(
			new AuthApiError('Invalid input', 422),
		);

		const {getByLabelText, getAllByRole, getByText} = renderLoggedOut(<Register/>);
		fill(getByLabelText, 'viewer@example.com', 'password123', 'password123');
		fireEvent.click(getAllByRole('button', {name: 'Create account'})[0]!);

		await waitFor(() => {
			expect(getByText(/invalid input/i)).toBeInTheDocument();
		});
	});

	it('disables button while registering', async () => {
		vi.mocked(register).mockImplementation(() => new Promise(() => {}));

		const {getByLabelText, getAllByRole} = renderLoggedOut(<Register/>);
		fill(getByLabelText, 'viewer@example.com', 'password123', 'password123');
		fireEvent.click(getAllByRole('button', {name: 'Create account'})[0]!);

		await waitFor(() => {
			expect(
				getAllByRole('button').some(
					button =>
						button.textContent?.includes('Creating account') &&
						button.hasAttribute('disabled'),
				),
			).toBe(true);
		});
	});

	it('offers a link back to login', () => {
		const {getByRole} = renderLoggedOut(<Register/>);

		expect(getByRole('link', {name: 'Log in'})).toHaveAttribute(
			'href',
			Routes.Login,
		);
	});
});
