import {describe, expect, it} from 'vitest';
import {render} from '@testing-library/preact';
import {NotFound} from './_404';
import {Routes} from '../routes';

describe('NotFound', () => {
	it('announces the missing page', () => {
		const {getByRole} = render(<NotFound/>);

		expect(getByRole('heading', {name: '404'})).toBeInTheDocument();
	});

	it('offers a way back home', () => {
		const {getByRole} = render(<NotFound/>);

		expect(getByRole('link', {name: 'Go home'})).toHaveAttribute(
			'href',
			Routes.Home,
		);
	});
});
