import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ServerStatus from './ServerStatus';
import { useAppStore } from '@/store';

describe('ServerStatus', () => {
  it('shows starting status', () => {
    useAppStore.setState({
      serverStatus: { state: 'starting' },
      config: { port: 29100, selected_printer: null },
      autostart: 'off',
    });

    render(<ServerStatus />);
    expect(screen.getByText('Starting…')).toBeInTheDocument();
    expect(screen.getByText('Restart')).toBeInTheDocument();
  });

  it('shows serving status', () => {
    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 29100, selected_printer: null },
      autostart: 'off',
    });

    render(<ServerStatus />);
    expect(screen.getByText('Server running')).toBeInTheDocument();
    expect(screen.getByText('Restart')).toBeInTheDocument();
  });

  it('shows standby status', () => {
    useAppStore.setState({
      serverStatus: { state: 'standby' },
      config: { port: 29100, selected_printer: null },
      autostart: 'off',
    });

    render(<ServerStatus />);
    expect(screen.getByText('Standing by')).toBeInTheDocument();
    expect(
      screen.getByText(
        "Another signed-in user's Dazzle is printing for this PC. This one takes over if that one stops. Print jobs show in that user's window."
      )
    ).toBeInTheDocument();
  });

  it('shows a failed status and its error', () => {
    useAppStore.setState({
      serverStatus: { state: 'failed', error: 'Port 9100 already in use' },
      config: { port: 9100, selected_printer: null },
      autostart: 'off',
    });

    render(<ServerStatus />);
    expect(screen.getByText('Not printing')).toBeInTheDocument();
    expect(screen.getByText('Port 9100 already in use')).toBeInTheDocument();
  });

  it('shows the port input with current value', () => {
    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 8080, selected_printer: null },
      autostart: 'off',
    });

    render(<ServerStatus />);
    const input = screen.getByDisplayValue('8080');
    expect(input).toBeInTheDocument();
  });

  it('calls restartServer on button click', async () => {
    const user = userEvent.setup();
    const restartServer = vi.fn().mockResolvedValue(undefined);

    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 29100, selected_printer: null },
      autostart: 'off',
      restartServer,
    });

    render(<ServerStatus />);
    await user.click(screen.getByText('Restart'));

    expect(restartServer).toHaveBeenCalled();
  });

  it('shows auto-start checkbox checked when this user enabled it', () => {
    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 29100, selected_printer: null },
      autostart: 'user',
    });

    render(<ServerStatus />);
    const checkbox = screen.getByLabelText('Launch at login');
    expect(checkbox).toBeChecked();
    expect(checkbox).toBeEnabled();
  });

  it('shows auto-start checkbox unchecked when autostart is off', () => {
    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 29100, selected_printer: null },
      autostart: 'off',
    });

    render(<ServerStatus />);
    const checkbox = screen.getByLabelText('Launch at login');
    expect(checkbox).not.toBeChecked();
  });

  it('shows launch at login locked on for all users', () => {
    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 29100, selected_printer: null },
      autostart: 'all_users',
    });

    render(<ServerStatus />);
    const checkbox = screen.getByLabelText('Launch at login (on for all users of this PC)');
    expect(checkbox).toBeChecked();
    expect(checkbox).toBeDisabled();
  });

  it('calls setAutostart when checkbox is toggled', async () => {
    const user = userEvent.setup();
    const setAutostart = vi.fn().mockResolvedValue(undefined);

    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 29100, selected_printer: null },
      autostart: 'off',
      setAutostart,
    });

    render(<ServerStatus />);
    const checkbox = screen.getByLabelText('Launch at login');
    await user.click(checkbox);

    expect(setAutostart).toHaveBeenCalledWith(true);
  });

  it('filters non-numeric characters from port input', async () => {
    const user = userEvent.setup();

    useAppStore.setState({
      serverStatus: { state: 'serving' },
      config: { port: 29100, selected_printer: null },
      autostart: 'off',
    });

    render(<ServerStatus />);
    const input = screen.getByDisplayValue('29100');
    await user.clear(input);
    await user.type(input, '80abc80');

    expect(input).toHaveValue('8080');
  });
});
