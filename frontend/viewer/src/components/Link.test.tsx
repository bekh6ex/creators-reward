import {describe, expect, it} from 'vitest';
import {render} from '@testing-library/preact';
import {Link} from './Link';

describe('Link', () => {
	it('renders an anchor with the given href', () => {
		const {getByRole} = render(<Link href="/dashboard">Dashboard</Link>);

		expect(getByRole('link', {name: /Dashboard/})).toHaveAttribute(
			'href',
			'/dashboard',
		);
	});

	it('opens internal links in the same tab', () => {
		const {getByRole} = render(<Link href="/dashboard">Dashboard</Link>);

		expect(getByRole('link')).not.toHaveAttribute('target');
	});

	it('opens external links in a new tab', () => {
		const {getByRole} = render(
			<Link href="https://example.com/article" external>
				Example
			</Link>,
		);

		expect(getByRole('link', {name: /Example/})).toHaveAttribute(
			'target',
			'_blank',
		);
	});
});
