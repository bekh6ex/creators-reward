import {describe, expect, it} from 'vitest';
import {render} from '@testing-library/preact';
import {Link} from './Link';

describe('Link', () => {
	it('renders an anchor pointing at the given href', () => {
		const {getByRole} = render(<Link href="/websites">Your websites</Link>);

		expect(getByRole('link', {name: 'Your websites'})).toHaveAttribute(
			'href',
			'/websites',
		);
	});

	it('opens an external link in a new tab', () => {
		const {getByRole} = render(
			<Link href="https://example.org" external>example.org</Link>,
		);

		expect(getByRole('link', {name: 'example.org'})).toHaveAttribute(
			'target',
			'_blank',
		);
	});

	it('keeps an internal link in the same tab', () => {
		const {getByRole} = render(<Link href="/payouts">Payouts</Link>);

		expect(getByRole('link', {name: 'Payouts'})).not.toHaveAttribute('target');
	});
});
