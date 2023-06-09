import React from 'react';
import {  Card, CardHeader, ListGroup, ListGroupItem } from 'reactstrap'
import 'bootstrap/dist/css/bootstrap.min.css';

function WordList(props) {
  let { words } = props


  return (
    <div style={{ paddingTop: '1rem', maxHeight: '10px'}}>
      <Card md="6">
        <CardHeader>
          Possible Answers:
        </CardHeader>
        <ListGroup flush>
          {
            words.map(word => {
              return (
                <ListGroupItem>
                  {word}
                </ListGroupItem>
            )
            })
          }
        </ListGroup>
      </Card>  
    </div>
  );
}

export default WordList;
