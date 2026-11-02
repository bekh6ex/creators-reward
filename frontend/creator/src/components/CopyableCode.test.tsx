import {afterEach, describe, expect, it, vi} from 'vitest';
import {fireEvent, render} from '@testing-library/preact';
import {CopyableCode} from './CopyableCode';

function stubClipboard(writeText: () => Promise<void>) {
	Object.defineProperty(navigator, 'clipboard', {
		value: {writeText},
		configurable: true,
	});
}

describe('CopyableCode', () => {
	afterEach(() => {
		Reflect.deleteProperty(navigator, 'clipboard');
	});

	it('shows the value in a read-only field', () => {
		const {getByLabelText} = render(
			<CopyableCode label="Meta tag" value="<meta content=abc>"/>,
		);

		const field = getByLabelText('Meta tag');
		expect(field).toHaveValue('<meta content=abc>');
		expect(field).toHaveAttribute('readonly');
	});

	it('copies the value and confirms it', async () => {
		const writeText = vi.fn(() => Promise.resolve());
		stubClipboard(writeText);
		const {findByText, getByRole} = render(
			<CopyableCode label="Meta tag" value="<meta content=abc>"/>,
		);

		fireEvent.click(getByRole('button', {name: 'Copy'}));

		expect(writeText).toHaveBeenCalledWith('<meta content=abc>');
		expect(await findByText('Copied to clipboard.')).toBeInTheDocument();
	});

	it('falls back to manual copying when the clipboard is unavailable', () => {
		const {getByRole, getByText} = render(
			<CopyableCode label="Meta tag" value="<meta content=abc>"/>,
		);

		fireEvent.click(getByRole('button', {name: 'Copy'}));

		expect(
			getByText('Select the text above and copy it manually.'),
		).toBeInTheDocument();
	});
});
