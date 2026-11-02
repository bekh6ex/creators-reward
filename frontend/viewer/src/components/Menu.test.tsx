import {describe, expect, it} from 'vitest';
import {render} from '@testing-library/preact';
import {Menu} from './Menu';
import {Routes} from '../routes';

describe('Menu', () => {
	it('renders a labelled navigation landmark', () => {
		const {getByRole} = render(<Menu/>);

		expect(getByRole('navigation', {name: 'Main'})).toBeInTheDocument();
	});

	it('links to the dashboard route', () => {
		const {getByRole} = render(<Menu/>);

		expect(getByRole('link', {name: 'Dashboard'})).toHaveAttribute(
			'href',
			Routes.Dashboard,
		);
	});

	it('links to the enabled donations route', () => {
		const {getByRole} = render(<Menu/>);

		expect(getByRole('link', {name: 'Enabled donations'})).toHaveAttribute(
			'href',
			Routes.EnabledDonations,
		);
	});
});
