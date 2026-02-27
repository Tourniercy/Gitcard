export interface Theme {
  name: string;
  background: string;
  backgroundBlur: string;
  border: string;
  title: string;
  text: string;
  muted: string;
  icon: string;
  ring: string;
}

export const defaultTheme: Theme = {
  name: 'default',
  background: 'rgba(255, 255, 255, 0.7)',
  backgroundBlur: 'rgba(255, 255, 255, 0.4)',
  border: 'rgba(0, 0, 0, 0.1)',
  title: '#1f2937',
  text: '#374151',
  muted: '#6b7280',
  icon: '#3b82f6',
  ring: '#3b82f6',
};

export const darkTheme: Theme = {
  name: 'dark',
  background: 'rgba(13, 17, 23, 0.8)',
  backgroundBlur: 'rgba(13, 17, 23, 0.5)',
  border: 'rgba(255, 255, 255, 0.1)',
  title: '#e6edf3',
  text: '#c9d1d9',
  muted: '#8b949e',
  icon: '#58a6ff',
  ring: '#58a6ff',
};

export const draculaTheme: Theme = {
  name: 'dracula',
  background: 'rgba(40, 42, 54, 0.8)',
  backgroundBlur: 'rgba(40, 42, 54, 0.5)',
  border: 'rgba(98, 114, 164, 0.3)',
  title: '#f8f8f2',
  text: '#f8f8f2',
  muted: '#6272a4',
  icon: '#bd93f9',
  ring: '#bd93f9',
};
