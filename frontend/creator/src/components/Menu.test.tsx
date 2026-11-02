import {describe, expect, it} from 'vitest';
import {render} from '@testing-library/preact';
import {Menu} from './Menu';
import {Routes} from '../routes';

describe('Menu', () => {
	it('links to every section of the portal', () => {
		const {getByRole} = render(<Menu/>);

		expect(getByRole('link', {name: 'Websites'})).toHaveAttribute(
			'href',
			Routes.Websites,
		);
		expect(getByRole('link', {name: 'Payouts'})).toHaveAttribute(
			'href',
			Routes.Payouts,
		);
		expect(getByRole('link', {name: 'Profile'})).toHaveAttribute(
			'href',
			Routes.Profile,
		);
	});

	it('is labelled so screen readers can skip it', () => {
		const {getByRole} = render(<Menu/>);

		expect(getByRole('navigation', {name: 'Main'})).toBeInTheDocument();
	});
});
