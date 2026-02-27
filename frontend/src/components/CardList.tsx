import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { CardType } from '../hooks/useCardConfig';
import { CardPreview } from './CardPreview';

interface CardListProps {
  cards: CardType[];
  onReorder: (cards: CardType[]) => void;
  buildSrc: (card: CardType) => string;
}

interface SortableCardProps {
  id: CardType;
  src: string;
}

function SortableCard({ id, src }: SortableCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div ref={setNodeRef} style={style} className="sortable-card" {...attributes} {...listeners}>
      <div className="sortable-card-handle">
        <span className="drag-icon">&#x2630;</span>
      </div>
      <CardPreview id={id} src={src} />
    </div>
  );
}

export function CardList({ cards, onReorder, buildSrc }: CardListProps) {
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = cards.indexOf(active.id as CardType);
      const newIndex = cards.indexOf(over.id as CardType);
      onReorder(arrayMove(cards, oldIndex, newIndex));
    }
  }

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <SortableContext items={cards} strategy={verticalListSortingStrategy}>
        <div className="card-list">
          {cards.map((card) => (
            <SortableCard key={card} id={card} src={buildSrc(card)} />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}
