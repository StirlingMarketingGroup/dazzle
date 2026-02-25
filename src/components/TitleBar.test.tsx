import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import TitleBar from './TitleBar';

describe('TitleBar', () => {
  it('renders the app name', () => {
    render(<TitleBar />);
    expect(screen.getByText('Dazzle')).toBeInTheDocument();
  });

  it('has a drag region', () => {
    const { container } = render(<TitleBar />);
    const dragRegion = container.querySelector('[data-tauri-drag-region]');
    expect(dragRegion).toBeInTheDocument();
  });
});
