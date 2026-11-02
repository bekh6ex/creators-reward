import {beforeEach, describe, expect, it, vi} from 'vitest';
import {fireEvent, waitFor} from '@testing-library/preact';
import {EnabledDonations} from './index';
import {renderLoggedIn} from '../../test/render';
import {disableDonations, fetchEnabledDonations} from '../../enabledDonationsApi';

vi.mock('../../enabledDonationsApi', () => ({
	fetchEnabledDonations: vi.fn(),
	disableDonations: vi.fn(),
}));

const DONATIONS = [
	{
		site: 'https://daily-ledger.example',
		enabledAt: '2023-09-01T10:00:00.000Z',
		recordIds: ['id-1', 'id-3'],
	},
	{
		site: 'https://kompas-weekly.example:8443',
		enabledAt: '2023-10-15T10:00:00.000Z',
		recordIds: ['id-2'],
	},
];

describe('EnabledDonations', () => {
	beforeEach(() => {
		vi.mocked(fetchEnabledDonations).mockReset();
		vi.mocked(fetchEnabledDonations).mockResolvedValue(DONATIONS);
		vi.mocked(disableDonations).mockReset();
		vi.mocked(disableDonations).mockResolvedValue(undefined);
	});

	it('shows a spinner until the enabled donations are decrypted', () => {
		vi.mocked(fetchEnabledDonations).mockReturnValue(new Promise(() => {}));
		const {getByLabelText} = renderLoggedIn(<EnabledDonations/>);

		expect(getByLabelText('Loading')).toBeInTheDocument();
	});

	it('renders a header plus one row per site with donations enabled', async () => {
		const {findAllByRole} = renderLoggedIn(<EnabledDonations/>);

		expect(await findAllByRole('row')).toHaveLength(3);
	});

	it('links each site by its host', async () => {
		const {findByRole} = renderLoggedIn(<EnabledDonations/>);

		expect(await findByRole('link', {name: 'kompas-weekly.example:8443'})).toHaveAttribute(
			'href',
			'https://kompas-weekly.example:8443',
		);
	});

	it('shows since when donations are enabled for each site', async () => {
		const {findByText} = renderLoggedIn(<EnabledDonations/>);

		expect(await findByText('01/09/2023')).toBeInTheDocument();
	});

	it('revokes every record of a site and removes its row', async () => {
		const {findAllByRole, getAllByRole, queryByRole} = renderLoggedIn(<EnabledDonations/>);

		fireEvent.click((await findAllByRole('button', {name: 'Disable donations'}))[0]!);

		await waitFor(() => expect(getAllByRole('row')).toHaveLength(2));
		expect(disableDonations).toHaveBeenCalledWith(DONATIONS[0]);
		expect(queryByRole('link', {name: 'daily-ledger.example'})).not.toBeInTheDocument();
	});

	it('keeps the row and says why when revoking fails', async () => {
		vi.mocked(disableDonations).mockRejectedValue(new Error('Disabling donations failed: 503'));
		const {findAllByRole, findByRole, getAllByRole} = renderLoggedIn(<EnabledDonations/>);

		fireEvent.click((await findAllByRole('button', {name: 'Disable donations'}))[0]!);

		expect(await findByRole('alert')).toHaveTextContent('Disabling donations failed: 503');
		expect(getAllByRole('row')).toHaveLength(3);
	});

	it('locks the buttons while a site is being disabled', async () => {
		vi.mocked(disableDonations).mockReturnValue(new Promise(() => {}));
		const {findAllByRole, getAllByRole} = renderLoggedIn(<EnabledDonations/>);

		fireEvent.click((await findAllByRole('button', {name: 'Disable donations'}))[0]!);

		await waitFor(() =>
			getAllByRole('button', {name: 'Disable donations'}).forEach(button =>
				expect(button).toBeDisabled(),
			),
		);
	});

	it('explains an empty list instead of showing an empty table', async () => {
		vi.mocked(fetchEnabledDonations).mockResolvedValue([]);
		const {findByRole, queryByRole} = renderLoggedIn(<EnabledDonations/>);

		expect(await findByRole('alert')).toHaveTextContent(
			'You have not enabled donations for any creator.',
		);
		expect(queryByRole('table')).not.toBeInTheDocument();
	});

	it('shows why the enabled donations could not be loaded', async () => {
		vi.mocked(fetchEnabledDonations).mockRejectedValue(new Error('Log in again.'));
		const {findByRole} = renderLoggedIn(<EnabledDonations/>);

		await waitFor(async () =>
			expect(await findByRole('alert')).toHaveTextContent('Log in again.'),
		);
	});
});
