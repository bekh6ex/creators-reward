import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {ResetPassword} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';

function fill(
	getByLabelText: (text: string) => HTMLElement,
	password: string,
	confirmation: string,
) {
	fireEvent.input(getByLabelText('New password'), {target: {value: password}});
	fireEvent.input(getByLabelText('Repeat password'), {
		target: {value: confirmation},
	});
}

describe('ResetPassword', () => {
	it('offers the form for a live token', () => {
		const {getByLabelText} = renderLoggedOut(<ResetPassword token="demo-reset-token"/>);

		expect(getByLabelText('New password')).toBeInTheDocument();
		expect(getByLabelText('Repeat password')).toBeInTheDocument();
	});

	it('refuses an expired token and offers a fresh link', () => {
		const {getByRole, queryByLabelText} = renderLoggedOut(
			<ResetPassword token="expired"/>,
		);

		expect(getByRole('alert')).toHaveTextContent('This reset link has expired.');
		expect(queryByLabelText('New password')).not.toBeInTheDocument();
		expect(getByRole('link', {name: 'Send another reset link'})).toHaveAttribute(
			'href',
			Routes.ForgotPassword,
		);
	});

	it('refuses a missing token', () => {
		const {getByRole} = renderLoggedOut(<ResetPassword token=""/>);

		expect(getByRole('alert')).toHaveTextContent('This reset link has expired.');
	});

	it('rejects a password shorter than eight characters', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(
			<ResetPassword token="demo-reset-token"/>,
		);

		fill(getByLabelText, 'short', 'short');
		fireEvent.click(getByRole('button', {name: 'Change password'}));

		expect(getByRole('alert')).toHaveTextContent('at least 8 characters');
	});

	it('rejects mismatched passwords', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(
			<ResetPassword token="demo-reset-token"/>,
		);

		fill(getByLabelText, 'secret123', 'secret124');
		fireEvent.click(getByRole('button', {name: 'Change password'}));

		expect(getByRole('alert')).toHaveTextContent('do not match');
	});

	it('confirms the change and offers the login page', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(
			<ResetPassword token="demo-reset-token"/>,
		);

		fill(getByLabelText, 'secret123', 'secret123');
		fireEvent.click(getByRole('button', {name: 'Change password'}));

		expect(getByRole('alert')).toHaveTextContent('Your password has been changed.');
		expect(getByRole('button', {name: 'Go to login'})).toBeInTheDocument();
	});
});
