import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {AuthenticatedHeader} from './AuthenticatedHeader';
import {renderLoggedIn} from '../test/render';

describe('AuthenticatedHeader', () => {
	it('shows who is logged in', () => {
		const {getByText} = renderLoggedIn(<AuthenticatedHeader/>);

		expect(getByText('owner@example.com')).toBeInTheDocument();
	});

	it('marks the portal apart from the reader app', () => {
		const {getByText} = renderLoggedIn(<AuthenticatedHeader/>);

		expect(getByText('Creator portal')).toBeInTheDocument();
	});

	it('drops the session when logging out', () => {
		const {getByRole, queryByText} = renderLoggedIn(<AuthenticatedHeader/>);

		fireEvent.click(getByRole('button', {name: 'Logout'}));

		expect(queryByText('owner@example.com')).not.toBeInTheDocument();
	});
});
