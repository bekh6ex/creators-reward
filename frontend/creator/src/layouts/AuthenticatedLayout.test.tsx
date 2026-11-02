import {describe, expect, it, vi} from 'vitest';
import {waitFor} from '@testing-library/preact';
import {AuthenticatedLayout} from './AuthenticatedLayout';
import {renderLoggedIn, renderLoggedOut} from '../test/render';
import {Routes} from '../routes';

describe('AuthenticatedLayout', () => {
	it('renders the page and the menu when logged in', () => {
		const {getByRole, getByText} = renderLoggedIn(
			<AuthenticatedLayout><p>Page body</p></AuthenticatedLayout>,
		);

		expect(getByText('Page body')).toBeInTheDocument();
		expect(getByRole('navigation', {name: 'Main'})).toBeInTheDocument();
	});

	it('waits for the session check before deciding', () => {
		const {getByLabelText, queryByText} = renderLoggedOut(
			<AuthenticatedLayout><p>Page body</p></AuthenticatedLayout>,
		);

		expect(getByLabelText('Loading')).toBeInTheDocument();
		expect(queryByText('Page body')).not.toBeInTheDocument();
	});

	it('sends a logged-out visitor home without showing the page', async () => {
		const pushState = vi.spyOn(window.history, 'pushState');

		const {queryByText} = renderLoggedOut(
			<AuthenticatedLayout><p>Page body</p></AuthenticatedLayout>,
		);

		await waitFor(() => expect(pushState).toHaveBeenCalledWith(null, '', Routes.Home));
		expect(queryByText('Page body')).not.toBeInTheDocument();
		pushState.mockRestore();
	});
});
