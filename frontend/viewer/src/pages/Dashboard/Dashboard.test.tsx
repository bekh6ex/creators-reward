import {describe, expect, it} from 'vitest';
import {fireEvent} from '@testing-library/preact';
import {Dashboard} from './index';
import {renderLoggedIn} from '../../test/render';

describe('Dashboard', () => {
	it('renders a header plus one row per viewed article', () => {
		const {getAllByRole} = renderLoggedIn(<Dashboard/>);

		expect(getAllByRole('row')).toHaveLength(5);
	});

	it('links each creator to its creator page', () => {
		const {getAllByRole, getByRole} = renderLoggedIn(<Dashboard/>);

		expect(getByRole('link', {name: 'Harbour Gazette'})).toHaveAttribute(
			'href',
			'/creator/1',
		);
		expect(getAllByRole('link', {name: 'Northern Post'})).toHaveLength(2);
	});

	it('opens article links in a new tab', () => {
		const {getByRole} = renderLoggedIn(<Dashboard/>);

		expect(
			getByRole('link', {name: /Rail strike/}),
		).toHaveAttribute('target', '_blank');
	});

	it('shows the cost of each view', () => {
		const {getAllByText, getByText} = renderLoggedIn(<Dashboard/>);

		expect(getAllByText('0.01 EUR')).toHaveLength(2);
		expect(getByText('0.02 EUR')).toBeInTheDocument();
	});

	it('totals the cost of the visible views', () => {
		const {getByText} = renderLoggedIn(<Dashboard/>);

		expect(getByText('0.07 EUR')).toBeInTheDocument();
	});

	it('offers a delete action per view', () => {
		const {getAllByRole} = renderLoggedIn(<Dashboard/>);

		expect(getAllByRole('button', {name: 'Delete'})).toHaveLength(4);
	});

	it('removes a view when it is deleted', () => {
		const {getAllByRole} = renderLoggedIn(<Dashboard/>);

		fireEvent.click(getAllByRole('button', {name: 'Delete'})[0]!);

		expect(getAllByRole('row')).toHaveLength(4);
	});

	it('offers creator and month filters', () => {
		const {getAllByRole, getByRole} = renderLoggedIn(<Dashboard/>);

		expect(getAllByRole('combobox')).toHaveLength(2);
		expect(getByRole('option', {name: 'By creator'})).toBeInTheDocument();
		expect(getByRole('option', {name: 'Harbour Gazette'})).toBeInTheDocument();
		expect(getByRole('option', {name: 'By month'})).toBeInTheDocument();
		expect(getByRole('option', {name: 'Nov 2023'})).toBeInTheDocument();
	});

	it('narrows the table when filtering by creator', () => {
		const {getAllByRole, getByRole} = renderLoggedIn(<Dashboard/>);

		fireEvent.change(getByRole('combobox', {name: 'Filter by creator'}), {
			target: {value: '2'},
		});

		expect(getAllByRole('row')).toHaveLength(3);
	});

	it('narrows the table when filtering by month', () => {
		const {getAllByRole, getByRole} = renderLoggedIn(<Dashboard/>);

		fireEvent.change(getByRole('combobox', {name: 'Filter by month'}), {
			target: {value: '2023-11'},
		});

		expect(getAllByRole('row')).toHaveLength(3);
	});

	it('explains an empty result instead of showing an empty table', () => {
		const {getByRole, queryByRole} = renderLoggedIn(<Dashboard/>);

		fireEvent.change(getByRole('combobox', {name: 'Filter by creator'}), {
			target: {value: '1'},
		});
		fireEvent.change(getByRole('combobox', {name: 'Filter by month'}), {
			target: {value: '2023-12'},
		});

		expect(queryByRole('table')).not.toBeInTheDocument();
		expect(getByRole('alert')).toHaveTextContent('No views match this filter.');
	});
});
