import {describe, expect, it} from 'vitest';
import {Home} from './index';
import {renderLoggedOut} from '../../test/render';
import {Routes} from '../../routes';

describe('Home', () => {
	it('shows the product name', () => {
		const {getByRole} = renderLoggedOut(<Home/>);

		expect(
			getByRole('heading', {name: "Creator's Reward"}),
		).toBeInTheDocument();
	});

	it('shows the login form', () => {
		const {getByLabelText, getByRole} = renderLoggedOut(<Home/>);

		expect(getByLabelText('Email')).toBeInTheDocument();
		expect(getByLabelText('Password')).toHaveAttribute('type', 'password');
		expect(getByRole('button', {name: 'Sign in'})).toBeInTheDocument();
	});

	it('links to registration', () => {
		const {getByRole} = renderLoggedOut(<Home/>);

		expect(getByRole('link', {name: 'Create one'})).toHaveAttribute(
			'href',
			Routes.Register,
		);
	});

	it('explains what the product does', () => {
		const {getByRole} = renderLoggedOut(<Home/>);

		expect(
			getByRole('heading', {name: 'Untrackable by design'}),
		).toBeInTheDocument();
	});
});
