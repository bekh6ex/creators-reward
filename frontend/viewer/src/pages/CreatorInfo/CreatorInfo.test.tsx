import {describe, expect, it} from 'vitest';
import {CreatorInfo} from './index';
import {renderWithLocation} from '../../test/render';

describe('CreatorInfo', () => {
	it('shows title and description for the given creator id', () => {
		const {getByRole, getByText} = renderWithLocation(
			<CreatorInfo id="1"/>,
		);

		expect(
			getByRole('heading', {name: 'Harbour Gazette'}),
		).toBeInTheDocument();
		expect(getByText(/Independent local newsroom/)).toBeInTheDocument();
	});

	it('renders a different creator for a different id', () => {
		const {getByRole, getByText} = renderWithLocation(
			<CreatorInfo id="2"/>,
		);

		expect(
			getByRole('heading', {name: 'Northern Post'}),
		).toBeInTheDocument();
		expect(getByText(/Daily reporting on national/)).toBeInTheDocument();
	});

	it('shows a not-found message for an unknown id', () => {
		const {getByRole, getByText} = renderWithLocation(
			<CreatorInfo id="999"/>,
		);

		expect(
			getByRole('heading', {name: 'Creator not found'}),
		).toBeInTheDocument();
		expect(
			getByText('No creator matches the id "999".'),
		).toBeInTheDocument();
	});
});
