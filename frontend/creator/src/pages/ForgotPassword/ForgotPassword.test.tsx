import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {ForgotPassword} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';

describe('ForgotPassword', () => {
	it('asks for the account email', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<ForgotPassword/>);

		expect(getByRole('heading', {name: 'Reset your password'})).toBeInTheDocument();
		expect(getByLabelText('Email')).toBeInTheDocument();
	});

	it('rejects an address that is not an email', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<ForgotPassword/>);

		fireEvent.input(getByLabelText('Email'), {target: {value: 'nope'}});
		fireEvent.click(getByRole('button', {name: 'Send reset link'}));

		expect(getByRole('alert')).toHaveTextContent('valid email address');
	});

	// Deliberately the same answer whether or not the address is registered:
	// otherwise this page tells a stranger which emails have accounts.
	it('confirms without revealing whether the account exists', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<ForgotPassword/>);

		fireEvent.input(getByLabelText('Email'), {
			target: {value: 'owner@example.com'},
		});
		fireEvent.click(getByRole('button', {name: 'Send reset link'}));

		expect(getByRole('alert')).toHaveTextContent(
			'If an account exists for owner@example.com',
		);
	});

	it('offers the reset page once the link is sent', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<ForgotPassword/>);

		fireEvent.input(getByLabelText('Email'), {
			target: {value: 'owner@example.com'},
		});
		fireEvent.click(getByRole('button', {name: 'Send reset link'}));

		expect(getByRole('link', {name: 'Open the reset page'})).toHaveAttribute(
			'href',
			'/reset-password/demo-reset-token',
		);
	});

	it('links back to login', () => {
		const {getByRole} = renderLoggedOut(<ForgotPassword/>);

		expect(getByRole('link', {name: 'Back to login'})).toHaveAttribute(
			'href',
			Routes.Login,
		);
	});
});
