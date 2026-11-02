import {describe, expect, it} from 'vitest';
import {Home} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';

describe('Home', () => {
	it('pitches the portal to people who publish', () => {
		const {getByRole} = renderLoggedOut(<Home/>);

		expect(
			getByRole('heading', {name: 'Get paid for what you publish'}),
		).toBeInTheDocument();
	});

	it('explains the three steps', () => {
		const {getByText} = renderLoggedOut(<Home/>);

		expect(getByText('Add your website')).toBeInTheDocument();
		expect(getByText('Readers donate what they choose')).toBeInTheDocument();
		expect(getByText('We transfer it to your bank')).toBeInTheDocument();
	});

	it('offers a login form', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<Home/>);

		expect(getByLabelText('Email')).toBeInTheDocument();
		expect(getByLabelText('Password')).toBeInTheDocument();
		expect(getByRole('button', {name: 'Log in'})).toBeInTheDocument();
	});

	it('links to registration and password recovery', () => {
		const {getByRole} = renderLoggedOut(<Home/>);

		expect(getByRole('link', {name: 'Create an account'})).toHaveAttribute(
			'href',
			Routes.Register,
		);
		expect(getByRole('link', {name: 'Forgot your password?'})).toHaveAttribute(
			'href',
			Routes.ForgotPassword,
		);
	});
});
