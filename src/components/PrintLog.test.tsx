import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import PrintLog from './PrintLog';
import { useAppStore } from '@/store';

describe('PrintLog', () => {
  it('shows empty state when no print jobs', () => {
    useAppStore.setState({ printJobs: [] });
    render(<PrintLog />);
    expect(screen.getByText(/no print jobs yet/i)).toBeInTheDocument();
  });

  it('renders print jobs', () => {
    useAppStore.setState({
      printJobs: [
        {
          id: '1',
          printer: 'Zebra ZD420',
          timestamp: 1700000000,
          status: 'completed',
          zpl_preview: '^XA^FDTest^FS^XZ',
        },
        {
          id: '2',
          printer: 'Brother QL-800',
          timestamp: 1700000060,
          status: 'failed',
          error: 'Printer offline',
        },
      ],
    });

    render(<PrintLog />);
    expect(screen.getByText('Zebra ZD420')).toBeInTheDocument();
    expect(screen.getByText('Brother QL-800')).toBeInTheDocument();
  });

  it('shows section heading', () => {
    useAppStore.setState({ printJobs: [] });
    render(<PrintLog />);
    expect(screen.getByText('Recent Jobs')).toBeInTheDocument();
  });
});
