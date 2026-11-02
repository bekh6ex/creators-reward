import {describe, expect, it} from 'vitest';
import {render} from '@testing-library/preact';
import {VerificationBadge} from './VerificationBadge';

describe('VerificationBadge', () => {
	it('names a verified site', () => {
		const {getByText} = render(<VerificationBadge status="verified"/>);

		expect(getByText('Verified')).toBeInTheDocument();
	});

	it('names a site that has never been checked', () => {
		const {getByText} = render(<VerificationBadge status="pending"/>);

		expect(getByText('Awaiting check')).toBeInTheDocument();
	});

	it('names a site whose tag was missing', () => {
		const {getByText} = render(<VerificationBadge status="failed"/>);

		expect(getByText('Tag not found')).toBeInTheDocument();
	});
});
