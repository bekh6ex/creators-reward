import {describe, expect, it} from 'vitest';
import {Websites} from './index';
import {renderLoggedIn} from '../../test/render';
import {Routes} from '../../routes';

describe('Websites', () => {
	it('renders a header plus one row per website', () => {
		const {getAllByRole} = renderLoggedIn(<Websites/>);

		expect(getAllByRole('row')).toHaveLength(4);
	});

	it('links each website to its own page', () => {
		const {getByRole} = renderLoggedIn(<Websites/>);

		expect(getByRole('link', {name: 'The Daily Ledger'})).toHaveAttribute(
			'href',
			'/websites/1',
		);
	});

	it('shows the verification state of every website', () => {
		const {getByText} = renderLoggedIn(<Websites/>);

		expect(getByText('Verified')).toBeInTheDocument();
		expect(getByText('Awaiting check')).toBeInTheDocument();
		expect(getByText('Tag not found')).toBeInTheDocument();
	});

	it('shows what each website has earned', () => {
		const {getAllByText, getByText} = renderLoggedIn(<Websites/>);

		expect(getByText('284.60 EUR')).toBeInTheDocument();
		expect(getAllByText('0.00 EUR')).toHaveLength(2);
	});

	it('offers a way to add another website', () => {
		const {getByRole} = renderLoggedIn(<Websites/>);

		expect(getByRole('link', {name: 'Add website'})).toHaveAttribute(
			'href',
			Routes.AddWebsite,
		);
	});

	it('explains the empty state instead of showing an empty table', () => {
		const {getByRole, queryByRole} = renderLoggedIn(<Websites/>, {websites: []});

		expect(queryByRole('table')).not.toBeInTheDocument();
		expect(getByRole('alert')).toHaveTextContent('You have not added a website yet.');
	});
});
