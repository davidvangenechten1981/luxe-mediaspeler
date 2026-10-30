import { MediaBrowser } from "../library/MediaBrowser";

export const VideosPage = () => (
  <MediaBrowser title="Videobibliotheek" base={{ kind: "video" }} emptyTitle="Nog geen video's" emptyText="Voeg een map met video's toe. VLC Ultimate scant hem op de achtergrond en toont de bestanden zodra ze gevonden zijn." />
);
export const MusicPage = () => (
  <MediaBrowser title="Muziekbibliotheek" base={{ kind: "audio" }} music defaultSort="artist" emptyTitle="Nog geen muziek" emptyText="Voeg een map met muziek toe. Artiest, album en jaar komen uit de tags in je bestanden." />
);
export const AllMediaPage = () => (
  <MediaBrowser title="Alle mediabestanden" base={{}} emptyTitle="Je bibliotheek is leeg" emptyText="Voeg een map toe om video's en muziek op één plek te beheren." />
);
export const FavoritesPage = () => (
  <MediaBrowser title="Favorieten" base={{ favorite_only: true }} emptyTitle="Nog geen favorieten" emptyText="Klik op het hartje bij een bestand om het hier te bewaren." />
);
export const RecentPage = () => (
  <MediaBrowser title="Recent afgespeeld" base={{ played_only: true }} defaultSort="played" defaultDescending music emptyTitle="Nog niets afgespeeld" emptyText="Alles wat je afspeelt, verschijnt hier, het nieuwste bovenaan." />
);
export const MostPlayedPage = () => (
  <MediaBrowser title="Meest gespeeld" base={{ played_only: true }} defaultSort="play_count" defaultDescending music emptyTitle="Nog niets afgespeeld" emptyText="Je meest afgespeelde bestanden komen hier vanzelf bovenaan." />
);
export const RecentlyAddedPage = () => (
  <MediaBrowser title="Recent toegevoegd" base={{}} defaultSort="added" defaultDescending music emptyTitle="Je bibliotheek is leeg" emptyText="Nieuwe bestanden verschijnen hier, het nieuwste bovenaan." />
);
