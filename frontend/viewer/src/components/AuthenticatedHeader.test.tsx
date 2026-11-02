import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {AuthenticatedHeader} from './AuthenticatedHeader';
import {renderLoggedIn} from '../test/render';

describe('AuthenticatedHeader', () => {
	it('shows the product name', () => {
		const {getByText} = renderLoggedIn(<AuthenticatedHeader/>);

		expect(getByText("Creator's Reward")).toBeInTheDocument();
	});

	it('shows the logged-in email', () => {
		const {getByText} = renderLoggedIn(<AuthenticatedHeader/>);

		expect(getByText('viewer@example.com')).toBeInTheDocument();
	});

	it('offers a logout action', () => {
		const {getByRole} = renderLoggedIn(<AuthenticatedHeader/>);

		expect(getByRole('button', {name: 'Logout'})).toBeInTheDocument();
	});

	it('sends the viewer home on logout', () => {
		const {getByRole, queryByText} = renderLoggedIn(<AuthenticatedHeader/>);

		fireEvent.click(getByRole('button', {name: 'Logout'}));

		expect(queryByText('viewer@example.com')).not.toBeInTheDocument();
	});
});
